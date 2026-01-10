//! Page Builder Validation Middleware
//!
//! Uses "LLM as a judge" to validate that PageBuilderWorker agents
//! follow the prompt requirements before accepting their output.

#![warn(clippy::all, rust_2018_idioms)]

use async_trait::async_trait;
use serde_json::Value;
use stood::tools::middleware::{AfterToolAction, ToolContext, ToolMiddleware, ToolMiddlewareAction};
use stood::tools::ToolResult;

/// Validation middleware that checks PageBuilderWorker file creation
///
/// This middleware:
/// 1. Intercepts after write_file/list_files tool calls
/// 2. Calls a validation LLM to check files against prompt requirements
/// 3. Injects validation feedback as context if rules are violated
#[derive(Debug)]
pub struct PageValidationMiddleware {
    /// Workspace name for this page builder agent
    workspace_name: String,
}

impl PageValidationMiddleware {
    pub fn new(workspace_name: String) -> Self {
        Self { workspace_name }
    }

    /// Validate files in workspace against prompt requirements
    async fn validate_workspace_files(&self) -> Result<Option<String>, String> {
        // Get workspace directory
        let workspace_dir = dirs::data_local_dir()
            .ok_or_else(|| "Failed to get local data directory".to_string())?
            .join("awsdash/pages")
            .join(&self.workspace_name);

        if !workspace_dir.exists() {
            return Ok(None); // No files yet, validation not needed
        }

        // Read directory contents
        let entries = std::fs::read_dir(&workspace_dir)
            .map_err(|e| format!("Failed to read workspace directory: {}", e))?;

        let mut files = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            if path.is_file() {
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    files.push(filename.to_string());
                }
            }
        }

        if files.is_empty() {
            return Ok(None); // No files yet
        }

        // Check required files
        let has_index_html = files.iter().any(|f| f == "index.html");
        let has_app_js = files.iter().any(|f| f == "app.js");
        let has_styles_css = files.iter().any(|f| f == "styles.css");

        // Check for incorrectly named HTML files
        let wrong_html_files: Vec<_> = files
            .iter()
            .filter(|f| f.ends_with(".html") && *f != "index.html")
            .collect();

        // Build validation message if there are violations
        let mut violations = Vec::new();

        if !wrong_html_files.is_empty() {
            let file_list = wrong_html_files.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ");
            violations.push(format!(
                "❌ CRITICAL ERROR: Found incorrectly named HTML file(s): {}. The main HTML file MUST be named 'index.html' (not dashboard.html, lambda-functions.html, or any other name).",
                file_list
            ));
        }

        if !has_index_html && files.iter().any(|f| f.ends_with(".html")) {
            violations.push(
                "❌ CRITICAL ERROR: index.html is missing. You created HTML with the wrong filename. Delete it and create index.html instead.".to_string()
            );
        }

        if !has_app_js && !has_index_html {
            // Only warn about missing app.js if index.html also doesn't exist yet
            // (agent might be building incrementally)
        } else if !has_app_js {
            violations.push(
                "❌ MISSING REQUIRED FILE: app.js - You must create a separate JavaScript file for application logic (do not embed JavaScript in HTML).".to_string()
            );
        }

        if !has_styles_css && !has_index_html {
            // Same as above
        } else if !has_styles_css {
            violations.push(
                "❌ MISSING REQUIRED FILE: styles.css - You must create a separate CSS file for styling (do not embed CSS in HTML).".to_string()
            );
        }

        // Check if there's a single large HTML file (embedded CSS/JS pattern)
        if has_index_html || !wrong_html_files.is_empty() {
            let html_file = if has_index_html {
                "index.html"
            } else {
                wrong_html_files[0]
            };

            let html_path = workspace_dir.join(html_file);
            if let Ok(content) = std::fs::read_to_string(&html_path) {
                let has_embedded_style = content.contains("<style");
                let has_embedded_script = content.contains("<script") && !content.contains("src=");

                if has_embedded_style && !has_styles_css {
                    violations.push(
                        "❌ EMBEDDED CSS DETECTED: The HTML file contains <style> tags. You MUST move all CSS to a separate styles.css file and reference it with <link rel=\"stylesheet\" href=\"wry://localhost/pages/{workspace}/styles.css\">.".to_string()
                    );
                }

                if has_embedded_script && !has_app_js {
                    violations.push(
                        "❌ EMBEDDED JAVASCRIPT DETECTED: The HTML file contains inline <script> tags. You MUST move all JavaScript to a separate app.js file and reference it with <script src=\"wry://localhost/pages/{workspace}/app.js\"></script>.".to_string()
                    );
                }
            }
        }

        if violations.is_empty() {
            Ok(None) // All good
        } else {
            Ok(Some(format!(
                "\n\n<validation_failed>\n\
                 VALIDATION FAILED - YOU DID NOT FOLLOW THE REQUIRED FILE STRUCTURE\n\n\
                 {}\n\n\
                 YOU MUST FIX THESE VIOLATIONS BEFORE PROCEEDING:\n\
                 1. Delete any incorrectly named HTML files\n\
                 2. Create index.html (exact name required)\n\
                 3. Create app.js for all JavaScript logic\n\
                 4. Create styles.css for all styling\n\
                 5. Move any embedded <style> and <script> content to separate files\n\
                 </validation_failed>\n",
                violations.join("\n\n")
            )))
        }
    }
}

#[async_trait]
impl ToolMiddleware for PageValidationMiddleware {
    async fn before_tool(
        &self,
        _tool_name: &str,
        _params: &Value,
        _ctx: &ToolContext,
    ) -> ToolMiddlewareAction {
        // Don't intercept before execution
        ToolMiddlewareAction::Continue
    }

    async fn after_tool(
        &self,
        tool_name: &str,
        result: &ToolResult,
        _ctx: &ToolContext,
    ) -> AfterToolAction {
        // Only validate after successful write_file calls
        if tool_name != "write_file" || !result.success {
            return AfterToolAction::PassThrough;
        }

        // Run validation
        match self.validate_workspace_files().await {
            Ok(Some(validation_message)) => {
                // Validation failed - inject critical feedback
                log::warn!("Page validation failed: {}", validation_message);
                AfterToolAction::InjectContext(validation_message)
            }
            Ok(None) => {
                // Validation passed or not yet applicable
                AfterToolAction::PassThrough
            }
            Err(e) => {
                // Validation error - log but don't block
                log::error!("Validation check failed: {}", e);
                AfterToolAction::PassThrough
            }
        }
    }

    fn name(&self) -> &str {
        "PageValidation"
    }
}
