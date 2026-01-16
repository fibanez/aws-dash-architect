//! Open Page - Preview page in webview
//!
//! This tool allows the Page Builder agent to open/preview the page
//! it's building in a webview window.
//!
//! Supports both disk-based and VFS-based workspaces.
//! VFS workspaces use the pattern: `vfs:{vfs_id}:{page_id}`

#![warn(clippy::all, rust_2018_idioms)]

use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use stood::tools::{Tool, ToolError, ToolResult};

use super::workspace::WorkspaceType;

/// Tool for opening/previewing a page in a webview
#[derive(Debug, Clone)]
pub struct OpenPageTool {
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
    ///
    /// For VFS workspaces, pass `vfs:{vfs_id}:{page_id}` as the page_name.
    /// For disk workspaces, pass a simple name like `my-dashboard`.
    pub fn new(page_name: &str) -> Result<Self> {
        // For disk-based workspaces, ensure the directory exists
        // VFS workspaces don't need disk directories
        if !page_name.starts_with("vfs:") {
            let workspace_root = dirs::data_local_dir()
                .ok_or_else(|| anyhow::anyhow!("Failed to get local data directory"))?
                .join("awsdash/pages")
                .join(page_name);

            std::fs::create_dir_all(&workspace_root)?;
        }

        Ok(Self {
            page_name: Some(page_name.to_string()),
        })
    }

    /// Create a new OpenPageTool that accepts page name as a parameter
    /// (used by TaskManager to open any tool)
    pub fn new_dynamic() -> Self {
        Self { page_name: None }
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

**CRITICAL**: Use the EXACT workspace_name returned from start_page_builder.
For VFS pages, this includes the full 'vfs:' prefix.

Examples:
- VFS page: {\"page_name\": \"vfs:abc123:vpc-explorer\"}
- Disk page: {\"page_name\": \"lambda-explorer\"}

The workspace_name is returned in the start_page_builder result - use it exactly as provided."
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

        // Parse workspace type (supports both VFS and disk-based workspaces)
        // We don't want from_workspace_name() to create disk directories here
        // since we're just checking for existence, not writing
        let workspace_type = if page_name.starts_with("vfs:") {
            // Parse VFS pattern: vfs:{vfs_id}:{page_id}
            let parts: Vec<&str> = page_name.splitn(3, ':').collect();
            if parts.len() != 3 || parts[1].is_empty() || parts[2].is_empty() {
                return Ok(ToolResult::error(format!(
                    "Invalid VFS workspace format '{}'. Expected 'vfs:{{vfs_id}}:{{page_id}}'",
                    page_name
                )));
            }
            WorkspaceType::Vfs {
                vfs_id: parts[1].to_string(),
                page_id: parts[2].to_string(),
            }
        } else {
            // Disk-based workspace
            let path = match dirs::data_local_dir() {
                Some(dir) => dir.join("awsdash/pages").join(&page_name),
                None => {
                    return Ok(ToolResult::error(
                        "Failed to get local data directory".to_string(),
                    ));
                }
            };
            WorkspaceType::Disk { path }
        };

        // Check if index.html exists (works for both VFS and disk)
        let index_exists = match workspace_type.exists("index.html") {
            Ok(exists) => exists,
            Err(e) => {
                return Ok(ToolResult::error(format!(
                    "Failed to check workspace: {}\nPage name: {}",
                    e, page_name
                )));
            }
        };

        if !index_exists {
            let workspace_desc = match &workspace_type {
                WorkspaceType::Vfs { vfs_id, page_id } => {
                    format!("VFS: {}:{}", vfs_id, page_id)
                }
                WorkspaceType::Disk { path } => path.display().to_string(),
            };
            return Ok(ToolResult::error(format!(
                "Tool preview failed: index.html not found in workspace.\n\
                 Create index.html first using write_file tool.\n\
                 Workspace: {}",
                workspace_desc
            )));
        }

        // Construct the wry protocol URL for the page
        // The webview protocol handler knows how to serve VFS files for vfs: prefixed names
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

                let page_path = match &workspace_type {
                    WorkspaceType::Vfs { vfs_id, page_id } => {
                        format!("VFS: {}:{}", vfs_id, page_id)
                    }
                    WorkspaceType::Disk { path } => path.display().to_string(),
                };

                let result = OpenPageResult {
                    status: "success".to_string(),
                    message,
                    page_name: page_name.clone(),
                    page_path,
                };

                match serde_json::to_value(result) {
                    Ok(json) => Ok(ToolResult::success(json)),
                    Err(e) => Ok(ToolResult::error(format!("Failed to serialize result: {}", e))),
                }
            }
            Err(e) => {
                let workspace_desc = match &workspace_type {
                    WorkspaceType::Vfs { vfs_id, page_id } => {
                        format!("VFS: {}:{}", vfs_id, page_id)
                    }
                    WorkspaceType::Disk { path } => path.display().to_string(),
                };
                Ok(ToolResult::error(format!(
                    "Failed to open page preview: {}\n\
                     Page name: {}\n\
                     Workspace: {}",
                    e, page_name, workspace_desc
                )))
            }
        }
    }
}
