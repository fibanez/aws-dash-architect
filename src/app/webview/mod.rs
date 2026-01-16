//! Webview integration module
//!
//! Provides functionality for spawning webviews in separate processes
//! with HTTP API communication between JavaScript and Rust.

use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, RwLock as StdRwLock,
};
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use wry::WebViewBuilder;

mod api_server;
mod commands;
mod page_manager;
mod pages_manager_window;

pub use api_server::ApiServer;
pub use page_manager::{DashPage, PageFolder, PageManager, get_page_manager};
pub use pages_manager_window::spawn_pages_manager_window;

/// Global API server info (set once at main process startup)
static GLOBAL_API_SERVER_INFO: StdRwLock<Option<(String, String)>> = StdRwLock::new(None);

/// Set the global API server info (URL and token)
pub fn set_api_server_info(base_url: String, token: String) {
    match GLOBAL_API_SERVER_INFO.write() {
        Ok(mut guard) => {
            tracing::info!("🔐 API server info configured: {}", base_url);
            *guard = Some((base_url, token));
        }
        Err(e) => {
            tracing::error!("Failed to set API server info: {}", e);
        }
    }
}

/// Get the global API server info
fn get_api_server_info() -> Option<(String, String)> {
    GLOBAL_API_SERVER_INFO.read().ok()?.clone()
}

/// DashApp JavaScript library (embedded)
const DASHAPP_JS: &str = include_str!("dashapp.js");

/// Get MIME type based on file extension
fn get_mime_type(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") => "text/html",
        Some("js") => "application/javascript",
        Some("css") => "text/css",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("svg") => "image/svg+xml",
        Some("woff") | Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("txt") => "text/plain",
        _ => "application/octet-stream",
    }
}

/// Sanitize and resolve path for serving Dash Page files from disk
///
/// Returns None if path is invalid (directory traversal, outside page directory, etc.)
fn sanitize_page_path(page_name: &str, file_path: &str) -> Option<PathBuf> {
    // Prevent directory traversal (../, /etc/passwd, etc.)
    if file_path.contains("..") || file_path.starts_with('/') {
        tracing::warn!("🚫 Rejected path with traversal attempt: {}", file_path);
        return None;
    }

    // Get pages base directory
    let base_dir = dirs::data_local_dir()?.join("awsdash/pages");
    let page_dir = base_dir.join(page_name);
    let full_path = page_dir.join(file_path);

    // Verify path is within page directory (additional safety check)
    if !full_path.starts_with(&page_dir) {
        tracing::warn!("🚫 Rejected path outside page directory: {}", file_path);
        return None;
    }

    Some(full_path)
}

pub fn spawn_webview_process(url: String, title: String) -> std::io::Result<()> {
    let current_exe = env::current_exe()?;

    Command::new(current_exe)
        .arg("--webview")
        .arg("--title")
        .arg(title)
        .arg("--url")
        .arg(url)
        .spawn()?;

    Ok(())
}

/// Open a page preview in a new webview window
///
/// This is used by the Page Builder agent to preview the page it's building.
/// The page must have an index.html file in its workspace directory.
///
/// Supports both disk-based pages (simple name like "my-dashboard") and
/// VFS-based pages (pattern like "vfs:{vfs_id}:{page_id}").
///
/// For VFS pages: The index.html is read from VFS here (main process), but
/// other assets (css, js) are fetched by the webview subprocess via HTTP
/// proxy to the API server's /vfs endpoint.
pub async fn open_page_preview(page_name: &str, _page_url: &str) -> anyhow::Result<()> {
    tracing::info!("Opening page preview for: {}", page_name);

    // Determine if this is a VFS-based or disk-based page
    let html = if page_name.starts_with("vfs:") {
        // VFS-based page: parse pattern vfs:{vfs_id}:{page_id}
        let parts: Vec<&str> = page_name.splitn(3, ':').collect();
        if parts.len() != 3 || parts[1].is_empty() || parts[2].is_empty() {
            return Err(anyhow::anyhow!(
                "Invalid VFS workspace format '{}'. Expected 'vfs:{{vfs_id}}:{{page_id}}'",
                page_name
            ));
        }

        let vfs_id = parts[1];
        let page_id = parts[2];
        let vfs_path = format!("/pages/{}/index.html", page_id);

        tracing::info!(
            "Reading index.html from VFS: vfs_id={}, path={}",
            vfs_id,
            vfs_path
        );

        // Read index.html from VFS (this runs in main process, so VFS is accessible)
        use crate::app::agent_framework::vfs::registry::with_vfs;
        let content = with_vfs(vfs_id, |vfs| vfs.read_file(&vfs_path).map(|c| c.to_vec()));

        match content {
            Some(Ok(bytes)) => String::from_utf8(bytes).map_err(|e| {
                anyhow::anyhow!("VFS index.html is not valid UTF-8: {}", e)
            })?,
            Some(Err(e)) => {
                return Err(anyhow::anyhow!(
                    "Failed to read index.html from VFS at '{}': {}",
                    vfs_path,
                    e
                ));
            }
            None => {
                return Err(anyhow::anyhow!(
                    "VFS not found for id '{}'. The agent session may have ended.",
                    vfs_id
                ));
            }
        }
    } else {
        // Disk-based page: read from filesystem
        let page_path = dirs::data_local_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not find local data directory"))?
            .join("awsdash/pages")
            .join(page_name)
            .join("index.html");

        if !page_path.exists() {
            return Err(anyhow::anyhow!(
                "Page index.html not found at {:?}",
                page_path
            ));
        }

        std::fs::read_to_string(&page_path)?
    };

    tracing::info!("Loaded page HTML ({} bytes)", html.len());

    // Spawn webview process with HTML
    // The subprocess custom protocol handler will proxy VFS asset requests
    // to the main process API server's /vfs endpoint
    let title = format!("Preview: {}", page_name.split(':').last().unwrap_or(page_name));
    spawn_webview_process_with_html(html, title)?;

    tracing::info!("Page preview webview spawned for: {}", page_name);

    Ok(())
}

pub fn spawn_webview_process_with_html(html: String, title: String) -> std::io::Result<()> {
    let current_exe = env::current_exe()?;

    // Get API server info (URL and token) to pass to webview
    let (api_url, api_token) = get_api_server_info()
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                "API server not initialized - call set_api_server_info() first",
            )
        })?;

    tracing::info!("Spawning webview with API URL: {}", api_url);

    // Pass HTML directly (not via HTTP) and API info via environment
    Command::new(current_exe)
        .arg("--webview")
        .arg("--title")
        .arg(title)
        .arg("--html")
        .arg(html)
        .env("AWSDASH_API_URL", api_url)
        .env("AWSDASH_API_TOKEN", api_token)
        .spawn()?;

    Ok(())
}

pub enum WebviewContent {
    Url(String),
    Html(String),
}

pub fn parse_webview_args(args: &[String]) -> Option<(WebviewContent, String)> {
    if !args.iter().any(|arg| arg == "--webview") {
        return None;
    }

    let mut title = "AWS Console".to_string();
    let mut url: Option<String> = None;
    let mut html: Option<String> = None;

    for i in 0..args.len() {
        if args[i] == "--title" && i + 1 < args.len() {
            title = args[i + 1].clone();
        } else if args[i] == "--url" && i + 1 < args.len() {
            url = Some(args[i + 1].clone());
        } else if args[i] == "--html" && i + 1 < args.len() {
            html = Some(args[i + 1].clone());
        }
    }

    // Prefer HTML over URL if both are provided
    let content = if let Some(h) = html {
        WebviewContent::Html(h)
    } else {
        WebviewContent::Url(url.unwrap_or_else(|| "https://console.aws.amazon.com/".to_string()))
    };

    Some((content, title))
}

pub fn run_webview(content: WebviewContent, title: String) -> wry::Result<()> {
    tracing::info!("run_webview called with title='{}'", title);

    match &content {
        WebviewContent::Url(url) => tracing::info!("Content: URL({})", url),
        WebviewContent::Html(html) => tracing::info!("Content: HTML({} bytes)", html.len()),
    }

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title(&title)
        .build(&event_loop)
        .unwrap();

    tracing::info!("Event loop and window created successfully");

    // Get API server info from environment (passed by main process)
    let api_url = env::var("AWSDASH_API_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:0".to_string());
    let api_token = env::var("AWSDASH_API_TOKEN")
        .unwrap_or_else(|_| String::new());

    tracing::info!("Webview configured to use API: {}", api_url);

    if api_token.is_empty() {
        tracing::warn!("No API token found - webview will not be able to make API requests");
    }

    // Build initialization script (inject dashapp.js with API URL and token)
    let init_script = format!(
        "{}\nwindow.__DASH_API_URL__ = '{}';\nwindow.__DASH_API_TOKEN__ = '{}';",
        DASHAPP_JS, api_url, api_token
    );
    tracing::info!("Initialization script prepared, length: {} bytes", init_script.len());

    // Window closing flag
    let is_closing = Arc::new(AtomicBool::new(false));

    tracing::info!("Creating WebViewBuilder");
    let mut builder = WebViewBuilder::new();

    // Add initialization script
    builder = builder.with_initialization_script(&init_script);

    // Add content - use custom protocol for HTML to get proper origin for fetch()
    builder = match &content {
        WebviewContent::Url(url) => {
            tracing::info!("Using URL content: {}", url);
            builder.with_url(url)
        }
        WebviewContent::Html(html) => {
            tracing::info!("Using embedded HTML ({} bytes) via wry://localhost protocol", html.len());
            let html_clone = html.clone();
            // Clone API info for use inside the closure
            let api_url_for_vfs = api_url.clone();
            let api_token_for_vfs = api_token.clone();

            // Register wry://localhost custom protocol
            // This gives the page origin "wry://localhost" which webkit allows to make fetch() requests
            builder = builder.with_custom_protocol("wry".into(), move |_webview_id, request| {
                let uri = request.uri().to_string();
                tracing::info!("📄 Custom protocol request: {}", uri);

                // Serve HTML at wry://localhost/ (and wry://localhost without trailing slash)
                if uri == "wry://localhost/" || uri == "wry://localhost" {
                    wry::http::Response::builder()
                        .header("Content-Type", "text/html")
                        .header("Access-Control-Allow-Origin", "*")
                        .body(html_clone.as_bytes().to_vec())
                        .unwrap()
                        .map(Into::into)
                }
                // Serve files from disk or VFS for paths like wry://localhost/pages/{name}/...
                else if let Some(path) = uri.strip_prefix("wry://localhost/pages/") {
                    // Parse page name and file path
                    // Format: pages/{page_name}/{file_path}
                    let parts: Vec<&str> = path.splitn(2, '/').collect();
                    if parts.len() != 2 {
                        tracing::warn!("❌ Invalid page path format: {}", path);
                        return wry::http::Response::builder()
                            .status(400)
                            .body(b"Invalid path format".to_vec())
                            .unwrap()
                            .map(Into::into);
                    }

                    let page_name = parts[0];
                    let file_path = parts[1];

                    // Check if this is a VFS-backed page (pattern: vfs:{vfs_id}:{page_id})
                    if page_name.starts_with("vfs:") {
                        tracing::info!("📂 Serving page file from VFS: {}", path);

                        // Parse VFS pattern: vfs:{vfs_id}:{page_id}
                        let vfs_parts: Vec<&str> = page_name.splitn(3, ':').collect();
                        if vfs_parts.len() != 3 {
                            tracing::warn!("❌ Invalid VFS page path format: {}", page_name);
                            return wry::http::Response::builder()
                                .status(400)
                                .body(b"Invalid VFS path format".to_vec())
                                .unwrap()
                                .map(Into::into);
                        }

                        let vfs_id = vfs_parts[1];
                        let vfs_page_id = vfs_parts[2];

                        // VFS is in the main process - we need to fetch via HTTP API
                        // API endpoint: GET /vfs/{vfs_id}/pages/{page_id}/{file_path}
                        let vfs_api_url = format!(
                            "{}/vfs/{}/pages/{}/{}",
                            api_url_for_vfs, vfs_id, vfs_page_id, file_path
                        );

                        tracing::info!("📡 Proxying VFS request to API: {}", vfs_api_url);

                        // Make blocking HTTP request to main process API server
                        let client = reqwest::blocking::Client::new();
                        match client
                            .get(&vfs_api_url)
                            .header("X-API-Token", &api_token_for_vfs)
                            .send()
                        {
                            Ok(response) => {
                                let status = response.status();
                                if status.is_success() {
                                    let content_type = response
                                        .headers()
                                        .get("Content-Type")
                                        .and_then(|v| v.to_str().ok())
                                        .unwrap_or("application/octet-stream")
                                        .to_string();

                                    match response.bytes() {
                                        Ok(bytes) => {
                                            tracing::info!(
                                                "✅ VFS proxy success: {} ({} bytes, type: {})",
                                                file_path,
                                                bytes.len(),
                                                content_type
                                            );
                                            wry::http::Response::builder()
                                                .header("Content-Type", content_type)
                                                .header("Access-Control-Allow-Origin", "*")
                                                .body(bytes.to_vec())
                                                .unwrap()
                                                .map(Into::into)
                                        }
                                        Err(e) => {
                                            tracing::warn!("❌ Failed to read VFS response body: {}", e);
                                            wry::http::Response::builder()
                                                .status(500)
                                                .body(format!("Failed to read response: {}", e).into_bytes())
                                                .unwrap()
                                                .map(Into::into)
                                        }
                                    }
                                } else {
                                    tracing::warn!("❌ VFS API returned {}: {}", status, file_path);
                                    wry::http::Response::builder()
                                        .status(status.as_u16())
                                        .body(format!("VFS file not found: {}", file_path).into_bytes())
                                        .unwrap()
                                        .map(Into::into)
                                }
                            }
                            Err(e) => {
                                tracing::warn!("❌ VFS API request failed: {}", e);
                                wry::http::Response::builder()
                                    .status(502)
                                    .body(format!("Failed to fetch from VFS API: {}", e).into_bytes())
                                    .unwrap()
                                    .map(Into::into)
                            }
                        }
                    } else {
                        // Disk-based page serving
                        tracing::info!("📂 Serving page file from disk: {}", path);

                        // Sanitize path and get full disk path
                        let disk_path = match sanitize_page_path(page_name, file_path) {
                            Some(p) => p,
                            None => {
                                tracing::warn!("❌ Path sanitization failed for: {}", path);
                                return wry::http::Response::builder()
                                    .status(403)
                                    .body(b"Forbidden: Invalid path".to_vec())
                                    .unwrap()
                                    .map(Into::into);
                            }
                        };

                        // Read file from disk
                        match std::fs::read(&disk_path) {
                            Ok(contents) => {
                                let mime_type = get_mime_type(file_path);
                                tracing::info!("✅ Served {} ({} bytes, type: {})", file_path, contents.len(), mime_type);

                                wry::http::Response::builder()
                                    .header("Content-Type", mime_type)
                                    .header("Access-Control-Allow-Origin", "*")
                                    .body(contents)
                                    .unwrap()
                                    .map(Into::into)
                            }
                            Err(e) => {
                                tracing::warn!("❌ Failed to read file {:?}: {}", disk_path, e);
                                wry::http::Response::builder()
                                    .status(404)
                                    .body(format!("File not found: {}", file_path).into_bytes())
                                    .unwrap()
                                    .map(Into::into)
                            }
                        }
                    }
                }
                else {
                    tracing::info!("❌ Custom protocol 404: {}", uri);
                    wry::http::Response::builder()
                        .status(404)
                        .body(Vec::new())
                        .unwrap()
                        .map(Into::into)
                }
            });

            // Navigate to wry://localhost/
            builder.with_url("wry://localhost/")
        }
    };

    tracing::info!("WebViewBuilder configured, about to build...");

    #[cfg(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    ))]
    let _webview = {
        tracing::info!("Building webview (non-Linux path)");
        builder.build(&window)?
    };

    #[cfg(not(any(
        target_os = "windows",
        target_os = "macos",
        target_os = "ios",
        target_os = "android"
    )))]
    let _webview = {
        tracing::info!("Building webview (Linux/GTK path)");
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        let vbox = window.default_vbox().unwrap();
        builder.build_gtk(vbox)?
    };

    tracing::info!("Webview built successfully");

    tracing::info!("Starting event loop");
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        if let Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } = event
        {
            tracing::info!("Window close requested");
            is_closing.store(true, Ordering::Relaxed);
            *control_flow = ControlFlow::Exit;
        }
    });
}

