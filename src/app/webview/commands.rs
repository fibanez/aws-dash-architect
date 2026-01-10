//! HTTP API command implementations for web applications
//!
//! The webview API exposes direct access to V8-bound functions for AWS operations.
//! Functions include: listAccounts, listRegions, loadCache, queryCachedResources,
//! getResourceSchema, showInExplorer, listBookmarks, queryBookmarks,
//! queryCloudWatchLogEvents, getCloudTrailEvents, and page management commands.

#![warn(clippy::all, rust_2018_idioms)]

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::app::agent_framework::v8_bindings::bindings::{
    accounts, regions, resources, cloudwatch_logs, cloudtrail_events,
};

/// Input arguments for openPage command
#[derive(Debug, Clone, serde::Deserialize)]
pub struct OpenPageArgs {
    /// Page/workspace name to open
    #[serde(rename = "pageName")]
    pub page_name: String,
    /// Optional message to display
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Open a page in a new webview window for preview
///
/// This allows pages to open themselves or other pages in new windows
pub async fn open_page(args: OpenPageArgs) -> Result<serde_json::Value> {
    tracing::info!("[WEBVIEW CMD] openPage(page_name: {}, message: {:?})",
        args.page_name, args.message);

    // Construct the wry protocol URL for the page
    let page_url = format!("wry://localhost/pages/{}/index.html", args.page_name);

    // Open the page in a new webview
    crate::app::webview::open_page_preview(&args.page_name, &page_url).await?;

    let message = args.message.unwrap_or_else(|| {
        format!("Page opened successfully in new window: {}", args.page_name)
    });

    tracing::info!("[WEBVIEW CMD] openPage() -> success");
    Ok(serde_json::json!({
        "status": "success",
        "message": message,
        "page_name": args.page_name,
        "page_url": page_url
    }))
}

// ============================================================================
// Direct V8 Binding Commands
// ============================================================================

/// List all configured AWS accounts
///
/// Calls the V8 binding function directly without spawning a V8 runtime
pub async fn list_accounts() -> Result<serde_json::Value> {
    tracing::info!("[WEBVIEW CMD] listAccounts()");

    let accounts = accounts::get_accounts_from_app()?;
    let result = serde_json::to_value(accounts)?;

    tracing::info!("[WEBVIEW CMD] listAccounts() -> {} accounts",
        result.as_array().map(|a| a.len()).unwrap_or(0));
    Ok(result)
}

/// List all AWS regions
///
/// Calls the V8 binding function directly without spawning a V8 runtime
pub async fn list_regions() -> Result<serde_json::Value> {
    tracing::info!("[WEBVIEW CMD] listRegions()");

    let regions = regions::get_regions();
    let result = serde_json::to_value(regions)?;

    tracing::info!("[WEBVIEW CMD] listRegions() -> {} regions",
        result.as_array().map(|a| a.len()).unwrap_or(0));
    Ok(result)
}

/// Load AWS resources into cache
///
/// Queries AWS resources and returns counts per scope combination
pub async fn load_cache(args: resources::LoadCacheArgs) -> Result<serde_json::Value> {
    tracing::info!("[WEBVIEW CMD] loadCache(resourceTypes: {:?})", args.resource_types);

    let result = resources::execute_load_cache(args)?;
    let json = serde_json::to_value(result)?;

    tracing::info!("[WEBVIEW CMD] loadCache() -> success");
    Ok(json)
}

/// Get resource schema for a resource type
///
/// Returns an example resource showing structure and available properties
pub async fn get_resource_schema(resource_type: String) -> Result<serde_json::Value> {
    tracing::info!("[WEBVIEW CMD] getResourceSchema({})", resource_type);

    let result = resources::execute_get_resource_schema(&resource_type)?;
    let status = result.status.clone();
    let json = serde_json::to_value(result)?;

    tracing::info!("[WEBVIEW CMD] getResourceSchema() -> {}", status);
    Ok(json)
}

/// Show resources in Explorer window
///
/// Opens the Explorer window with dynamic configuration
pub async fn show_in_explorer(args: resources::ShowInExplorerArgs) -> Result<serde_json::Value> {
    tracing::info!("[WEBVIEW CMD] showInExplorer(title: {:?})", args.title);

    // Enqueue action for Explorer window
    crate::app::resource_explorer::enqueue_explorer_action(
        crate::app::resource_explorer::ExplorerAction::OpenWithConfig(args),
    );

    let result = resources::ShowInExplorerResult {
        status: "success".to_string(),
        message: Some("Explorer window action enqueued".to_string()),
        resources_displayed: None,
    };

    tracing::info!("[WEBVIEW CMD] showInExplorer() -> success");
    Ok(serde_json::to_value(result)?)
}

/// Query cached resources
///
/// Returns actual resource objects from cache for filtering/analysis
pub async fn query_cached_resources(args: resources::QueryCachedResourcesArgs) -> Result<serde_json::Value> {
    tracing::info!("[WEBVIEW CMD] queryCachedResources(resourceTypes: {:?})", args.resource_types);

    let result = resources::execute_query_cached_resources(args)?;
    let json = serde_json::to_value(result)?;

    tracing::info!("[WEBVIEW CMD] queryCachedResources() -> {} resources",
        json.get("count").and_then(|c| c.as_u64()).unwrap_or(0));
    Ok(json)
}

/// List all bookmarks
///
/// Returns a flat list of all saved bookmarks
pub async fn list_bookmarks() -> Result<serde_json::Value> {
    tracing::info!("[WEBVIEW CMD] listBookmarks()");

    use crate::app::resource_explorer::get_global_bookmark_manager;
    use crate::app::resource_explorer::unified_query::BookmarkInfo;

    let manager = get_global_bookmark_manager()
        .ok_or_else(|| anyhow::anyhow!("Bookmark manager not initialized"))?;

    let bookmarks: Vec<BookmarkInfo> = manager
        .read()
        .map_err(|e| anyhow::anyhow!("Failed to read bookmarks: {}", e))?
        .get_bookmarks()
        .iter()
        .map(BookmarkInfo::from)
        .collect();

    let result = serde_json::to_value(bookmarks)?;

    tracing::info!("[WEBVIEW CMD] listBookmarks() -> {} bookmarks",
        result.as_array().map(|a| a.len()).unwrap_or(0));
    Ok(result)
}

/// Query bookmarks args
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryBookmarksArgs {
    /// Bookmark ID to query
    pub bookmark_id: String,
    /// Optional detail level
    pub options: Option<resources::QueryBookmarksArgs>,
}

/// Query a bookmark
///
/// Executes a bookmark's saved query and returns resources
pub async fn query_bookmarks(args: QueryBookmarksArgs) -> Result<serde_json::Value> {
    tracing::info!("[WEBVIEW CMD] queryBookmarks({})", args.bookmark_id);

    // Use the internal async function directly
    let options = args.options.unwrap_or(resources::QueryBookmarksArgs { detail: None });
    let result = resources::query_bookmark_internal(&args.bookmark_id, options).await?;
    let json = serde_json::to_value(result)?;

    tracing::info!("[WEBVIEW CMD] queryBookmarks() -> success");
    Ok(json)
}

/// Query CloudWatch Log events
///
/// Queries CloudWatch Logs for analysis and monitoring
pub async fn query_cloudwatch_log_events(
    args: cloudwatch_logs::QueryCloudWatchLogEventsArgs
) -> Result<serde_json::Value> {
    tracing::info!("[WEBVIEW CMD] queryCloudWatchLogEvents(logGroup: {}, account: {}, region: {})",
        args.log_group_name, args.account_id, args.region);

    let result = cloudwatch_logs::query_cloudwatch_logs_internal(args).await?;
    let json = serde_json::to_value(result)?;

    tracing::info!("[WEBVIEW CMD] queryCloudWatchLogEvents() -> {} events",
        json.get("totalEvents").and_then(|c| c.as_u64()).unwrap_or(0));
    Ok(json)
}

/// Get CloudTrail events
///
/// Queries CloudTrail events for governance and compliance analysis
pub async fn get_cloudtrail_events(
    args: cloudtrail_events::GetCloudTrailEventsArgs
) -> Result<serde_json::Value> {
    tracing::info!("[WEBVIEW CMD] getCloudTrailEvents(account: {}, region: {})",
        args.account_id, args.region);

    let result = cloudtrail_events::get_cloudtrail_events_internal(args).await?;
    let json = serde_json::to_value(result)?;

    tracing::info!("[WEBVIEW CMD] getCloudTrailEvents() -> {} events",
        json.get("totalEvents").and_then(|c| c.as_u64()).unwrap_or(0));
    Ok(json)
}

// ============================================================================
// Page Management Commands
// ============================================================================

/// Information about a page for listing
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    /// Page/workspace name (directory name)
    pub name: String,
    /// Creation timestamp (earliest file creation time)
    pub created_at: i64,
    /// Last modification timestamp (most recent file modification)
    pub last_modified: i64,
    /// Total size of all files in bytes
    pub total_size: u64,
    /// Number of files in the page
    pub file_count: usize,
}

/// Get the pages directory path
fn get_pages_dir() -> Result<PathBuf> {
    let pages_dir = dirs::data_local_dir()
        .ok_or_else(|| anyhow::anyhow!("Failed to get local data directory"))?
        .join("awsdash/pages");
    Ok(pages_dir)
}

/// List all pages with metadata
///
/// Reads the pages directory and returns info about each page
pub async fn list_pages() -> Result<serde_json::Value> {
    tracing::info!("[WEBVIEW CMD] listPages()");

    let pages_dir = get_pages_dir()?;

    let mut pages: Vec<PageInfo> = Vec::new();

    if pages_dir.exists() {
        let entries = std::fs::read_dir(&pages_dir)?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                let name = entry.file_name().to_string_lossy().to_string();

                // Skip hidden directories
                if name.starts_with('.') {
                    continue;
                }

                // Collect file metadata
                let mut created_at: i64 = i64::MAX;
                let mut last_modified: i64 = 0;
                let mut total_size: u64 = 0;
                let mut file_count: usize = 0;

                if let Ok(files) = std::fs::read_dir(&path) {
                    for file_entry in files.flatten() {
                        let file_path = file_entry.path();
                        if file_path.is_file() {
                            file_count += 1;

                            if let Ok(metadata) = file_path.metadata() {
                                total_size += metadata.len();

                                // Get creation time
                                if let Ok(created) = metadata.created() {
                                    if let Ok(duration) = created.duration_since(std::time::UNIX_EPOCH) {
                                        let ts = duration.as_secs() as i64;
                                        if ts < created_at {
                                            created_at = ts;
                                        }
                                    }
                                }

                                // Get modification time
                                if let Ok(modified) = metadata.modified() {
                                    if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                                        let ts = duration.as_secs() as i64;
                                        if ts > last_modified {
                                            last_modified = ts;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // If no files, use directory creation time
                if created_at == i64::MAX {
                    if let Ok(dir_meta) = path.metadata() {
                        if let Ok(created) = dir_meta.created() {
                            if let Ok(duration) = created.duration_since(std::time::UNIX_EPOCH) {
                                created_at = duration.as_secs() as i64;
                            }
                        }
                    }
                }

                // If still no created_at, use 0
                if created_at == i64::MAX {
                    created_at = 0;
                }

                pages.push(PageInfo {
                    name,
                    created_at,
                    last_modified,
                    total_size,
                    file_count,
                });
            }
        }
    }

    // Sort by last_modified descending (most recent first)
    pages.sort_by(|a, b| b.last_modified.cmp(&a.last_modified));

    let result = serde_json::to_value(&pages)?;
    tracing::info!("[WEBVIEW CMD] listPages() -> {} pages", pages.len());
    Ok(result)
}

/// Input arguments for deletePage command
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeletePageArgs {
    /// Page/workspace name to delete
    pub page_name: String,
}

/// Delete a page and all its files
///
/// Removes the page directory and all contents
pub async fn delete_page(args: DeletePageArgs) -> Result<serde_json::Value> {
    tracing::info!("[WEBVIEW CMD] deletePage(page_name: {})", args.page_name);

    let pages_dir = get_pages_dir()?;
    let page_path = pages_dir.join(&args.page_name);

    // Validate the path is within pages directory (prevent path traversal)
    if !page_path.starts_with(&pages_dir) {
        return Err(anyhow::anyhow!("Invalid page name: path traversal not allowed"));
    }

    if !page_path.exists() {
        return Err(anyhow::anyhow!("Page not found: {}", args.page_name));
    }

    if !page_path.is_dir() {
        return Err(anyhow::anyhow!("Path is not a directory: {}", args.page_name));
    }

    // Remove the directory and all contents
    std::fs::remove_dir_all(&page_path)?;

    tracing::info!("[WEBVIEW CMD] deletePage() -> success");
    Ok(serde_json::json!({
        "status": "success",
        "message": format!("Page '{}' deleted successfully", args.page_name),
        "page_name": args.page_name
    }))
}

/// Input arguments for viewPage command
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ViewPageArgs {
    /// Page/workspace name to view
    pub page_name: String,
}

/// View a page in a new webview window
///
/// Opens the page for preview (same as openPage but with different semantics)
pub async fn view_page(args: ViewPageArgs) -> Result<serde_json::Value> {
    tracing::info!("[WEBVIEW CMD] viewPage(page_name: {})", args.page_name);

    let pages_dir = get_pages_dir()?;
    let page_path = pages_dir.join(&args.page_name);

    // Validate the path is within pages directory
    if !page_path.starts_with(&pages_dir) {
        return Err(anyhow::anyhow!("Invalid page name: path traversal not allowed"));
    }

    if !page_path.exists() {
        return Err(anyhow::anyhow!("Page not found: {}", args.page_name));
    }

    // Construct the wry protocol URL for the page
    let page_url = format!("wry://localhost/pages/{}/index.html", args.page_name);

    // Open the page in a new webview
    crate::app::webview::open_page_preview(&args.page_name, &page_url).await?;

    tracing::info!("[WEBVIEW CMD] viewPage() -> success");
    Ok(serde_json::json!({
        "status": "success",
        "message": format!("Page '{}' opened in new window", args.page_name),
        "page_name": args.page_name,
        "page_url": page_url
    }))
}

/// Input arguments for editPage command
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditPageArgs {
    /// Page/workspace name to edit
    pub page_name: String,
}

/// Edit a page using the TaskManager agent
///
/// Sends a UI event to open the page in edit mode with a TaskManager agent.
/// The agent will have the page workspace set and prompt the user for changes.
pub async fn edit_page(args: EditPageArgs) -> Result<serde_json::Value> {
    tracing::info!("[WEBVIEW CMD] editPage(page_name: {})", args.page_name);

    let pages_dir = get_pages_dir()?;
    let page_path = pages_dir.join(&args.page_name);

    // Validate the path is within pages directory
    if !page_path.starts_with(&pages_dir) {
        return Err(anyhow::anyhow!("Invalid page name: path traversal not allowed"));
    }

    if !page_path.exists() {
        return Err(anyhow::anyhow!("Page not found: {}", args.page_name));
    }

    // Send UI event to open page for editing
    use crate::app::agent_framework::ui::agent_events::{send_ui_event, AgentUIEvent};

    send_ui_event(AgentUIEvent::open_page_for_edit(args.page_name.clone()))
        .map_err(|e| anyhow::anyhow!("Failed to send edit page event: {}", e))?;

    tracing::info!("[WEBVIEW CMD] editPage() -> success");
    Ok(serde_json::json!({
        "status": "success",
        "message": format!("Opening editor for page '{}'", args.page_name),
        "page_name": args.page_name
    }))
}

/// Input arguments for renamePage command
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenamePageArgs {
    /// Current page/workspace name
    pub old_name: String,
    /// New page/workspace name
    pub new_name: String,
}

/// Rename a page (folder)
///
/// Renames the page directory to a new name and updates all wry:// URLs
/// in the page files to reference the new name.
pub async fn rename_page(args: RenamePageArgs) -> Result<serde_json::Value> {
    tracing::info!("[WEBVIEW CMD] renamePage(old_name: {}, new_name: {})",
        args.old_name, args.new_name);

    // Validate new name is not empty and doesn't contain invalid characters
    let new_name = args.new_name.trim();
    if new_name.is_empty() {
        return Err(anyhow::anyhow!("New name cannot be empty"));
    }

    // Check for invalid characters (only allow alphanumeric, dash, underscore)
    if !new_name.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ' ') {
        return Err(anyhow::anyhow!("Invalid name: only alphanumeric characters, dashes, underscores, and spaces are allowed"));
    }

    let pages_dir = get_pages_dir()?;
    let old_path = pages_dir.join(&args.old_name);
    let new_path = pages_dir.join(new_name);

    // Validate paths are within pages directory (prevent path traversal)
    if !old_path.starts_with(&pages_dir) || !new_path.starts_with(&pages_dir) {
        return Err(anyhow::anyhow!("Invalid page name: path traversal not allowed"));
    }

    if !old_path.exists() {
        return Err(anyhow::anyhow!("Page not found: {}", args.old_name));
    }

    if !old_path.is_dir() {
        return Err(anyhow::anyhow!("Path is not a directory: {}", args.old_name));
    }

    if new_path.exists() {
        return Err(anyhow::anyhow!("A page with the name '{}' already exists", new_name));
    }

    // Update wry:// URLs in all files before renaming the directory
    let files_updated = update_wry_urls_in_directory(&old_path, &args.old_name, new_name)?;
    tracing::info!("[WEBVIEW CMD] renamePage() updated {} files with new URLs", files_updated);

    // Rename the directory
    std::fs::rename(&old_path, &new_path)?;

    tracing::info!("[WEBVIEW CMD] renamePage() -> success");
    Ok(serde_json::json!({
        "status": "success",
        "message": format!("Page renamed from '{}' to '{}' ({} files updated)", args.old_name, new_name, files_updated),
        "old_name": args.old_name,
        "new_name": new_name,
        "files_updated": files_updated
    }))
}

/// Recursively update wry:// URLs in all text files within a directory
///
/// Replaces `wry://localhost/pages/{old_name}/` with `wry://localhost/pages/{new_name}/`
fn update_wry_urls_in_directory(dir: &PathBuf, old_name: &str, new_name: &str) -> Result<usize> {
    let mut files_updated = 0;

    // Text file extensions to process
    let text_extensions = ["html", "htm", "js", "css", "json", "txt", "md", "xml", "svg"];

    // Build the URL patterns to find and replace
    let old_url_pattern = format!("wry://localhost/pages/{}/", old_name);
    let new_url_pattern = format!("wry://localhost/pages/{}/", new_name);

    // Recursively process all files in the directory
    fn process_dir(
        dir: &PathBuf,
        old_pattern: &str,
        new_pattern: &str,
        text_extensions: &[&str],
        files_updated: &mut usize
    ) -> Result<()> {
        let entries = std::fs::read_dir(dir)?;

        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                // Recursively process subdirectories
                process_dir(&path, old_pattern, new_pattern, text_extensions, files_updated)?;
            } else if path.is_file() {
                // Check if this is a text file we should process
                let should_process = path
                    .extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| text_extensions.contains(&ext.to_lowercase().as_str()))
                    .unwrap_or(false);

                if should_process {
                    // Read the file
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        // Check if the file contains the old URL pattern
                        if content.contains(old_pattern) {
                            // Replace and write back
                            let new_content = content.replace(old_pattern, new_pattern);
                            std::fs::write(&path, new_content)?;
                            *files_updated += 1;
                            tracing::debug!("[WEBVIEW CMD] Updated URLs in: {:?}", path);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    process_dir(dir, &old_url_pattern, &new_url_pattern, &text_extensions, &mut files_updated)?;

    Ok(files_updated)
}
