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

use crate::app::agent_framework::agent_instance::AgentInstance;
use crate::app::agent_framework::conversation::{ConversationMessage, ConversationRole};
use crate::app::agent_framework::status_display::ProcessingStatusWidget;

/// Worker display info for inline rendering in conversation
#[derive(Debug, Clone)]
pub struct InlineWorkerDisplay {
    /// Short description (e.g., "Finding S3 buckets")
    pub short_description: String,
    /// Currently running tool (if any)
    pub current_tool: Option<String>,
    /// Whether worker is still running
    pub is_running: bool,
    /// Whether worker succeeded (only valid when not running)
    pub success: bool,
    /// Path to worker's log file
    pub log_path: Option<PathBuf>,
    /// Total tokens used by this worker (cumulative across model calls)
    pub total_tokens: u32,
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
                        if let Some(clicked_path) = render_inline_workers(ui, workers) {
                            worker_log_clicked = Some(clicked_path);
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
/// Returns the log path if a worker's Log button was clicked.
fn render_inline_workers(ui: &mut Ui, workers: &[InlineWorkerDisplay]) -> Option<PathBuf> {
    let mut log_to_open: Option<PathBuf> = None;

    for worker in workers {
        ui.horizontal(|ui| {
            // Format token count for display
            let token_str = format_tokens(worker.total_tokens);

            // Status indicator and description with token count
            let status_text = if worker.is_running {
                if let Some(tool) = &worker.current_tool {
                    // Running with tool: "Finding S3 buckets > execute_javascript (1.2K)"
                    format!("  {} > {} ({})", worker.short_description, tool, token_str)
                } else {
                    // Running, waiting: "Finding S3 buckets ... (45.6K)"
                    format!("  {} ... ({})", worker.short_description, token_str)
                }
            } else {
                // Completed: "Finding S3 buckets (done, 0.8M)"
                let status = if worker.success { "done" } else { "failed" };
                format!("  {} ({}, {})", worker.short_description, status, token_str)
            };

            // Use subtle color for workers
            let color = if worker.is_running {
                egui::Color32::GRAY
            } else if worker.success {
                egui::Color32::from_rgb(100, 160, 100) // Subtle green
            } else {
                egui::Color32::from_rgb(180, 100, 100) // Subtle red
            };

            ui.label(RichText::new(&status_text).color(color).small());

            // Log button
            if let Some(log_path) = &worker.log_path {
                if ui.small_button("Log").clicked() {
                    log_to_open = Some(log_path.clone());
                }
            }
        });
    }

    log_to_open
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

            // User message - with visual anchor, theme-adaptive color
            // Use ">" as visual anchor (egui has very limited emoji support)
            let strong_color = ui.visuals().strong_text_color();
            ui.label(
                RichText::new(format!("> {}", message.content))
                    .color(strong_color) // Theme-adaptive strong color
                    .size(21.0)
                    .font(egui::FontId::proportional(21.0)),
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
