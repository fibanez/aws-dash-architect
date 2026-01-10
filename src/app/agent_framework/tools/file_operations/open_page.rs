//! Open Page - Preview page in webview
//!
//! This tool allows the Page Builder agent to open/preview the page
//! it's building in a webview window.

#![warn(clippy::all, rust_2018_idioms)]

use anyhow::{Context, Result};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use stood::tools::{Tool, ToolError, ToolResult};

/// Tool for opening/previewing a page in a webview
#[derive(Debug, Clone)]
pub struct OpenPageTool {
    /// Optional pre-configured workspace root (for PageBuilderWorker)
    workspace_root: Option<PathBuf>,
    /// Optional pre-configured page name (for PageBuilderWorker)
    page_name: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct OpenPageParams {
    /// Page name/workspace name to open (required if not pre-configured)
    #[serde(default)]
    page_name: Option<String>,
    /// Optional message to display (default: "Opening tool preview...")
    #[serde(default)]
    message: Option<String>,
}

#[derive(Debug, Serialize)]
struct OpenPageResult {
    status: String,
    message: String,
    page_name: String,
    page_path: String,
}

impl OpenPageTool {
    /// Create a new OpenPageTool with a pre-configured page name
    /// (used by PageBuilderWorker)
    pub fn new(page_name: &str) -> Result<Self> {
        let workspace_root = dirs::data_local_dir()
            .context("Failed to get local data directory")?
            .join("awsdash/pages")
            .join(page_name);

        // Ensure workspace exists
        std::fs::create_dir_all(&workspace_root)?;

        Ok(Self {
            workspace_root: Some(workspace_root),
            page_name: Some(page_name.to_string()),
        })
    }

    /// Create a new OpenPageTool that accepts page name as a parameter
    /// (used by TaskManager to open any tool)
    pub fn new_dynamic() -> Self {
        Self {
            workspace_root: None,
            page_name: None,
        }
    }
}

#[async_trait]
impl Tool for OpenPageTool {
    fn name(&self) -> &str {
        "open_page"
    }

    fn description(&self) -> &str {
        if self.page_name.is_some() {
            // Pre-configured mode (PageBuilderWorker)
            "Open the page in a webview for preview/testing.

BEFORE calling this tool, verify:
1. All asset URLs use wry://localhost/pages/{workspace}/filename pattern
2. All dashApp API calls have 'await' keyword
3. All event listeners are attached in DOMContentLoaded
4. HTML element IDs match JavaScript getElementById() calls
5. Property paths were validated with execute_javascript

After preview opens, check browser console (F12) for JavaScript errors.

Call this after creating or updating files to test the page in action."
        } else {
            // Dynamic mode (TaskManager)
            "Open a completed page in a webview for preview/testing.

Provide the page_name parameter with the workspace name of the page to open.
This is typically the workspace name from a completed PageBuilder worker.

Example: {\"page_name\": \"lambda-explorer\"}"
        }
    }

    fn parameters_schema(&self) -> serde_json::Value {
        if self.page_name.is_some() {
            // Pre-configured mode - page_name not needed
            serde_json::json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "Optional message to display (default: 'Opening tool preview...')"
                    }
                }
            })
        } else {
            // Dynamic mode - page_name required
            serde_json::json!({
                "type": "object",
                "required": ["page_name"],
                "properties": {
                    "page_name": {
                        "type": "string",
                        "description": "The workspace name of the page to open (e.g., 'lambda-explorer')"
                    },
                    "message": {
                        "type": "string",
                        "description": "Optional message to display (default: 'Opening tool preview...')"
                    }
                }
            })
        }
    }

    async fn execute(
        &self,
        parameters: Option<serde_json::Value>,
        _agent_context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        // Parse parameters
        let params: OpenPageParams = match parameters {
            Some(p) => serde_json::from_value(p).map_err(|e| ToolError::InvalidParameters {
                message: format!("Failed to parse parameters: {}", e),
            })?,
            None => OpenPageParams {
                page_name: None,
                message: None,
            },
        };

        // Determine page_name (from pre-config or parameters)
        let page_name = if let Some(ref name) = self.page_name {
            name.clone()
        } else if let Some(ref name) = params.page_name {
            name.clone()
        } else {
            return Err(ToolError::InvalidParameters {
                message: "page_name parameter is required".to_string(),
            });
        };

        // Determine workspace_root
        let workspace_root = if let Some(ref root) = self.workspace_root {
            root.clone()
        } else {
            dirs::data_local_dir()
                .context("Failed to get local data directory")
                .map_err(|e| ToolError::ExecutionFailed {
                    message: format!("Failed to get data directory: {}", e),
                })?
                .join("awsdash/pages")
                .join(&page_name)
        };

        // Check if index.html exists
        let index_path = workspace_root.join("index.html");
        if !index_path.exists() {
            return Ok(ToolResult::error(format!(
                "Tool preview failed: index.html not found in workspace.\n\
                 Create index.html first using write_file tool.\n\
                 Workspace: {}",
                workspace_root.display()
            )));
        }

        // Construct the wry protocol URL for the page
        let page_url = format!("wry://localhost/pages/{}/index.html", page_name);

        // Send request to open the page in a webview
        // This uses the global webview manager to spawn a new webview
        match crate::app::webview::open_page_preview(&page_name, &page_url).await {
            Ok(_) => {
                let message = params.message.unwrap_or_else(|| {
                    format!(
                        "Page preview opened successfully!\n\
                         The page is now running in a webview window.\n\
                         URL: {}",
                        page_url
                    )
                });

                let result = OpenPageResult {
                    status: "success".to_string(),
                    message,
                    page_name: page_name.clone(),
                    page_path: workspace_root.display().to_string(),
                };

                match serde_json::to_value(result) {
                    Ok(json) => Ok(ToolResult::success(json)),
                    Err(e) => Ok(ToolResult::error(format!("Failed to serialize result: {}", e))),
                }
            }
            Err(e) => Ok(ToolResult::error(format!(
                "Failed to open page preview: {}\n\
                 Page name: {}\n\
                 Workspace: {}",
                e, page_name,
                workspace_root.display()
            ))),
        }
    }
}
