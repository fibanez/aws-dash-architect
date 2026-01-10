//! Agent Chat UI
//!
//! Renders the agent conversation interface with markdown support for assistant responses.
//!
//! ## Features
//!
//! - **Markdown Rendering**: Assistant messages are automatically rendered as markdown
//!   when they contain code blocks, headers, lists, bold text, or links
//! - **Syntax Highlighting**: Code blocks use language-aware coloring via syntect
//! - **Height-constrained Layout**: Scroll area prevents window auto-growth
//! - **Per-agent Scroll Position**: Each agent maintains independent scroll state
//! - **Lock-free Rendering**: Data collected before rendering to avoid UI blocking
//! - **Fixed Input Area**: Input box stays at bottom regardless of content size
//!
//! ## Message Display
//!
//! - User messages: Plain text with ">" prefix and theme-adaptive strong color
//! - Assistant messages: Markdown-rendered if detected, otherwise plain text

use egui::{RichText, ScrollArea, Ui};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use crate::app::agent_framework::core::instance::AgentInstance;
use crate::app::agent_framework::conversation::{ConversationMessage, ConversationRole};
use crate::app::agent_framework::status_display::ProcessingStatusWidget;
use crate::perf_checkpoint;

/// Status of a tool call within a worker
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCallStatus {
    /// Tool is currently executing
    Running,
    /// Tool completed successfully
    Success,
    /// Tool failed with error message
    Failed(String),
}

/// Display record of a single tool call by a worker agent
#[derive(Debug, Clone)]
pub struct ToolCallDisplayRecord {
    /// Tool name (e.g., "execute_javascript")
    pub tool_name: String,
    /// Human-readable intent (e.g., "Creating HTML structure")
    pub intent: String,
    /// Tokens used for this specific tool call
    pub tokens: Option<u32>,
    /// Current status of this tool call
    pub status: ToolCallStatus,
    /// When this tool call started
    pub started_at: Instant,
    /// When this tool call completed (if finished)
    pub completed_at: Option<Instant>,
}

/// Request to open a tool viewer for a ToolBuilder workspace
#[derive(Debug, Clone)]
pub struct PageViewRequest {
    /// Name of the workspace to open
    pub workspace_name: String,
}

/// Actions requested from worker rendering
#[derive(Debug, Clone)]
pub enum WorkerActionRequest {
    /// Open a worker's log file
    OpenLog(PathBuf),
    /// Open a ToolBuilder's viewer
    OpenPageView(PageViewRequest),
}

/// Worker display info for inline rendering in conversation
#[derive(Debug, Clone)]
pub struct InlineWorkerDisplay {
    /// Short description (e.g., "Finding S3 buckets")
    pub short_description: String,
    /// History of all tool calls made by this worker
    pub tool_calls: Vec<ToolCallDisplayRecord>,
    /// Whether worker is still running
    pub is_running: bool,
    /// Whether worker succeeded (only valid when not running)
    pub success: bool,
    /// Path to worker's log file
    pub log_path: Option<PathBuf>,
    /// Whether this is a ToolBuilder worker
    pub is_tool_builder: bool,
    /// Workspace name (for ToolBuilder workers)
    pub workspace_name: Option<String>,
}

/// Format token count with K/M suffixes for readability
///
/// - Under 1000: "234" (no suffix)
/// - 1K to 999K: "1.2K", "45.6K" (one decimal)
/// - 1M+: "1.2M", "0.8M" (one decimal)
fn format_tokens(tokens: u32) -> String {
    if tokens >= 1_000_000 {
        format!("{:.1}M", tokens as f64 / 1_000_000.0)
    } else if tokens >= 1_000 {
        format!("{:.1}K", tokens as f64 / 1_000.0)
    } else {
        tokens.to_string()
    }
}

/// Check if content appears to be markdown
///
/// Uses simple heuristics - looks for common markdown patterns:
/// - Headers (# ## ###)
/// - Code blocks (```)
/// - Lists (* - 1.)
/// - Bold/italic (**text**)
/// - Links [text](url)
fn looks_like_markdown(content: &str) -> bool {
    let patterns = [
        "```",    // Code blocks
        "\n# ",   // H1 header
        "\n## ",  // H2 header
        "\n### ", // H3 header
        "\n* ",   // Unordered list
        "\n- ",   // Unordered list
        "\n1. ",  // Ordered list
        "**",     // Bold
        "](http", // Links
    ];

    patterns.iter().any(|p| content.contains(p))
}

/// Render the agent chat interface
///
/// This function applies hard-won layout lessons:
/// - Collects data before rendering (no lock holding during UI)
/// - Uses height-constrained scroll area
/// - Maintains per-agent scroll position
/// - Fixed input area at bottom
///
/// Parameters:
/// - `inline_workers`: Optional map of message_index -> workers to display inline after each message
///
/// Returns: `(should_send, log_clicked, clear_clicked, terminate_clicked, stop_clicked, worker_log_clicked)`
/// where `worker_log_clicked` is the log path if a worker's log button was clicked
pub fn render_agent_chat(
    ui: &mut Ui,
    agent: &mut AgentInstance,
    input_text: &mut String,
    markdown_cache: &mut CommonMarkCache,
    status_widget: &mut ProcessingStatusWidget,
    inline_workers: Option<&HashMap<usize, Vec<InlineWorkerDisplay>>>,
) -> (bool, bool, bool, bool, bool, Option<PathBuf>) {
    // Collect data before rendering to avoid holding locks during UI rendering
    let is_processing = agent.is_processing();
    let can_cancel = agent.can_cancel();
    let status_message = agent.status_message().map(|s| s.to_string());
    let messages: Vec<ConversationMessage> = agent.messages().iter().cloned().collect();
    let agent_id = agent.id();

    // Update widget from agent's processing phase
    let phase = agent.processing_phase().clone();
    status_widget.set_phase(phase);

    // Set detail text from status message
    if let Some(ref msg) = status_message {
        status_widget.set_detail(Some(msg.clone()));
    } else {
        status_widget.set_detail(None);
    }

    // Calculate max height: reserve space for status + input + buttons + separators
    // Status: ~20px, Input (3 lines): ~70px, Buttons: ~30px, Separators: ~30px = ~150px total
    let conversation_max_height = ui.available_height() - 150.0;

    // Track if a worker log button was clicked
    let mut worker_log_clicked: Option<PathBuf> = None;

    // Scrollable conversation area with critical constraints + auto-scroll
    // Use both() to enable horizontal scrolling for wide content like tables
    ScrollArea::both()
        .id_salt(("conversation_scroll", agent_id)) // Per-agent scroll position
        .auto_shrink([false, false]) // Don't shrink - prevents collapse
        .max_height(conversation_max_height) // Cap height - prevents vertical auto-growth
        .stick_to_bottom(true) // Auto-scroll to show latest messages
        .show(ui, |ui| {
            // No placeholder message - just show empty space when no messages
            for (index, message) in messages.iter().enumerate() {
                render_message(ui, message, markdown_cache);

                // Render inline workers that were spawned by this message
                if let Some(workers_map) = inline_workers {
                    if let Some(workers) = workers_map.get(&index) {
                        if let Some(action) = render_inline_workers(ui, workers) {
                            match action {
                                WorkerActionRequest::OpenLog(path) => {
                                    worker_log_clicked = Some(path);
                                }
                                WorkerActionRequest::OpenPageView(request) => {
                                    let workspace_name = request.workspace_name.clone();

                                    tracing::info!(
                                        target: "agent::ui_events::page_open",
                                        workspace_name = %workspace_name,
                                        "Opening page preview webview for PageBuilder workspace"
                                    );

                                    // Spawn async task to open page preview
                                    // This calls the same function used by the open_page agent tool
                                    tokio::spawn(async move {
                                        let page_url = format!("wry://localhost/pages/{}/index.html", workspace_name);

                                        match crate::app::webview::open_page_preview(&workspace_name, &page_url).await {
                                            Ok(_) => {
                                                tracing::info!(
                                                    target: "agent::ui_events::page_open",
                                                    workspace_name = %workspace_name,
                                                    "Page preview webview opened successfully"
                                                );
                                            }
                                            Err(e) => {
                                                tracing::error!(
                                                    target: "agent::ui_events::page_open",
                                                    workspace_name = %workspace_name,
                                                    error = %e,
                                                    "Failed to open page preview webview"
                                                );
                                            }
                                        }
                                    });
                                }
                            }
                        }
                    }
                }

                ui.add_space(1.0);
            }
        });

    // Status line with animated widget
    // Widget handles its own space reservation and animation
    status_widget.show(ui);

    // Input area - always at bottom, fixed height
    let mut should_send = false;
    let mut keep_focus = false;

    ui.vertical(|ui| {
        // Multi-line input with 3 rows minimum
        let input_response = ui.add(
            egui::TextEdit::multiline(input_text)
                .desired_rows(3)
                .desired_width(f32::INFINITY),
        );

        // Track if input had focus before (for loose focus behavior)
        let had_focus = input_response.has_focus();

        // Enter key to send message (without Shift) while input has focus
        // Shift+Enter adds a newline (default TextEdit behavior)
        if had_focus
            && ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift)
            && !input_text.is_empty()
            && !is_processing
        {
            should_send = true;
            keep_focus = true; // Keep focus after sending
        }

        // Send button
        ui.horizontal(|ui| {
            let send_enabled = !input_text.is_empty() && !is_processing;
            if ui
                .add_enabled(send_enabled, egui::Button::new("Send"))
                .clicked()
            {
                should_send = true;
                keep_focus = had_focus; // Only maintain focus if it was already focused
            }
        });

        // Request focus for next frame only if we're maintaining it (loose focus - not forcing)
        if keep_focus && had_focus {
            ui.memory_mut(|mem| mem.request_focus(input_response.id));
        }
    });

    // Controls section
    ui.add_space(10.0);

    // Action buttons
    let (log_clicked, clear_clicked, terminate_clicked, stop_clicked) = ui
        .horizontal(|ui| {
            // Stop button - only enabled when processing and cancellation is available
            let stop_enabled = is_processing && can_cancel;
            let stop_clicked = ui
                .add_enabled(stop_enabled, egui::Button::new("Stop"))
                .clicked();

            ui.separator();

            // Log button
            let log_clicked = ui.button("Log").clicked();

            ui.separator();

            // Clear button
            let clear_clicked = ui.button("Clear Conversation").clicked();

            ui.separator();

            // Terminate button
            let terminate_clicked = ui.button("Terminate Agent").clicked();

            (log_clicked, clear_clicked, terminate_clicked, stop_clicked)
        })
        .inner;

    (
        should_send,
        log_clicked,
        clear_clicked,
        terminate_clicked,
        stop_clicked,
        worker_log_clicked,
    )
}

/// Render inline workers for a specific message
///
/// Returns a worker action request if a button was clicked (Log or Open Tool).
fn render_inline_workers(ui: &mut Ui, workers: &[InlineWorkerDisplay]) -> Option<WorkerActionRequest> {
    let mut action_request: Option<WorkerActionRequest> = None;

    perf_checkpoint!(
        "ui.render_workers",
        &format!("count={}", workers.len())
    );

    // Reduce spacing between workers for single-spaced appearance
    let original_spacing = ui.spacing().item_spacing.y;
    ui.spacing_mut().item_spacing.y = 2.0;

    for worker in workers {
        perf_checkpoint!(
            "ui.render_worker",
            &format!("desc={} calls={} running={}",
                worker.short_description, worker.tool_calls.len(), worker.is_running)
        );
        // Worker header with short description and action buttons
        ui.horizontal(|ui| {
            ui.label(RichText::new(format!("  {}", worker.short_description)).strong());

            // Log button
            if let Some(log_path) = &worker.log_path {
                if ui.small_button("Log").clicked() {
                    action_request = Some(WorkerActionRequest::OpenLog(log_path.clone()));
                }
            }

            // Open Tool button for completed ToolBuilder workers
            if worker.is_tool_builder && !worker.is_running && worker.success {
                if let Some(workspace_name) = &worker.workspace_name {
                    if ui.small_button("Open Tool").clicked() {
                        action_request = Some(WorkerActionRequest::OpenPageView(PageViewRequest {
                            workspace_name: workspace_name.clone(),
                        }));
                    }
                }
            }
        });

        // Show tool call history with indentation in a scroll area
        if !worker.tool_calls.is_empty() {
            // Wrap in scroll area with reduced height
            ScrollArea::vertical()
                .max_height(50.0)
                .auto_shrink([false, true])
                .stick_to_bottom(true)
                .id_salt(("tool_calls_scroll", &worker.short_description))
                .show(ui, |ui| {
                    for tool_call in &worker.tool_calls {
                        ui.horizontal(|ui| {
                            // Indent tool calls
                            ui.add_space(20.0);

                            // Status icon
                            let status_icon = match &tool_call.status {
                                ToolCallStatus::Running => "...",
                                ToolCallStatus::Success => "[done]",
                                ToolCallStatus::Failed(_) => "[FAIL]",
                            };

                            // Token string for this call
                            let tokens_str = if let Some(t) = tool_call.tokens {
                                format!(" ({})", format_tokens(t))
                            } else {
                                String::new()
                            };

                            // Choose color based on status
                            let color = match &tool_call.status {
                                ToolCallStatus::Running => egui::Color32::GRAY,
                                ToolCallStatus::Success => egui::Color32::from_rgb(100, 160, 100),
                                ToolCallStatus::Failed(_) => egui::Color32::from_rgb(180, 100, 100),
                            };

                            // Render: "    [done] Creating HTML structure (5.1K)"
                            // Custom rendering without label padding for compact display
                            let text = format!("{} {}{}", status_icon, tool_call.intent, tokens_str);
                            let font_id = egui::FontId::proportional(10.0); // small size
                            let galley = ui.fonts(|f| f.layout_no_wrap(text, font_id.clone(), color));

                            // Allocate space for the text without any padding
                            let (rect, _response) = ui.allocate_exact_size(
                                galley.size(),
                                egui::Sense::hover(),
                            );

                            // Draw the text directly at the allocated position
                            ui.painter().galley(rect.min, galley, color);
                        });
                    }
                });

            // Total tokens summary
            ui.horizontal(|ui| {
                ui.add_space(20.0);
                let total: u32 = worker
                    .tool_calls
                    .iter()
                    .filter_map(|c| c.tokens)
                    .sum();
                ui.label(
                    RichText::new(format!("Total: {}", format_tokens(total)))
                        .weak()
                        .small(),
                );
            });
        }
    }

    // Restore original spacing
    ui.spacing_mut().item_spacing.y = original_spacing;

    action_request
}

/// Render a single message
///
/// User messages are rendered as plain text with a ">" prefix, preceded by an empty line.
/// Assistant messages are rendered as markdown if they contain markdown patterns,
/// otherwise as plain text.
fn render_message(ui: &mut Ui, message: &ConversationMessage, cache: &mut CommonMarkCache) {
    match message.role {
        ConversationRole::User => {
            // Add empty line before user messages for visual separation
            ui.add_space(8.0);
            ui.label("");

            // User message - with visual anchor, theme-adaptive lighter color
            // Use ">" as visual anchor (egui has very limited emoji support)
            // Smaller font and lighter color than assistant responses
            let weak_color = ui.visuals().weak_text_color();
            ui.label(
                RichText::new(format!("> {}", message.content))
                    .color(weak_color) // Theme-adaptive weak color (lighter than assistant)
                    .size(14.0)
                    .font(egui::FontId::proportional(14.0)),
            );
        }
        ConversationRole::Assistant => {
            // Assistant message - render as markdown if detected, otherwise plain text
            // Use message timestamp as unique ID to avoid duplicate widget IDs for tables
            let message_id = message.timestamp.timestamp_millis();
            ui.push_id(message_id, |ui| {
                if looks_like_markdown(&message.content) {
                    CommonMarkViewer::new().show(ui, cache, &message.content);
                } else {
                    ui.label(&message.content);
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_rendering() {
        // Basic smoke test - actual rendering tested via integration tests
        let user_msg = ConversationMessage::user("Hello");
        assert_eq!(user_msg.role, ConversationRole::User);

        let assistant_msg = ConversationMessage::assistant("Hi there!");
        assert_eq!(assistant_msg.role, ConversationRole::Assistant);
    }

    #[test]
    fn test_format_tokens_under_thousand() {
        // Under 1000: no suffix
        assert_eq!(format_tokens(0), "0");
        assert_eq!(format_tokens(1), "1");
        assert_eq!(format_tokens(234), "234");
        assert_eq!(format_tokens(999), "999");
    }

    #[test]
    fn test_format_tokens_thousands() {
        // 1K to 999K: one decimal with K suffix
        assert_eq!(format_tokens(1000), "1.0K");
        assert_eq!(format_tokens(1200), "1.2K");
        assert_eq!(format_tokens(1234), "1.2K"); // rounds down
        assert_eq!(format_tokens(45600), "45.6K");
        assert_eq!(format_tokens(100000), "100.0K");
        assert_eq!(format_tokens(999999), "1000.0K"); // edge case near million
    }

    #[test]
    fn test_format_tokens_millions() {
        // 1M+: one decimal with M suffix
        assert_eq!(format_tokens(1_000_000), "1.0M");
        assert_eq!(format_tokens(1_200_000), "1.2M");
        assert_eq!(format_tokens(45_600_000), "45.6M");
        assert_eq!(format_tokens(100_000_000), "100.0M");
    }
}
