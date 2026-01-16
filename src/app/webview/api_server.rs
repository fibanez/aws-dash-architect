//! HTTP API Server for webview communication
//!
//! This module implements a lightweight HTTP server that runs in the main process
//! and executes API commands from webview processes using the main
//! process's AWS client and cache.
//!
//! # Architecture
//!
//! ```text
//! Webview Process              Main Process
//! ┌──────────────┐            ┌──────────────────────────┐
//! │ JavaScript   │  HTTP POST │ API Server               │
//! │ dashApp.xxx()├───────────►│ validate token           │
//! └──────────────┘            │ execute with             │
//!                             │ GLOBAL_AWS_CLIENT        │
//!                             │ GLOBAL_EXPLORER_STATE    │
//!                             └──────────────────────────┘
//! ```
//!
//! # Security
//!
//! - Random API token generated on server startup
//! - Token passed to webview via environment variable
//! - Each request validated with X-API-Token header
//! - Stops random programs from calling API
//! - Future: Developer mode toggle for open API

#![warn(clippy::all, rust_2018_idioms)]

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};

/// API server configuration and state
pub struct ApiServer {
    /// Random token for authentication
    api_token: String,
    /// Port the server is listening on
    port: u16,
    /// Server shutdown handle
    shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
}

/// Shared state passed to all request handlers
#[derive(Clone)]
struct AppState {
    api_token: String,
}

/// Generic API request body
#[derive(Debug, Deserialize)]
struct ApiRequest {
    /// Command name
    cmd: String,
    /// Command payload (JSON)
    payload: serde_json::Value,
}

/// Generic API response
#[derive(Debug, Serialize)]
struct ApiResponse {
    /// Success or error
    success: bool,
    /// Response data (JSON) or error message
    data: serde_json::Value,
}

impl ApiServer {
    /// Start the API server on a random available port
    ///
    /// Returns the server instance with port and token for passing to webview
    pub async fn start() -> anyhow::Result<Arc<RwLock<Self>>> {
        // Generate random API token (32 bytes hex = 64 characters)
        let api_token = generate_api_token();

        info!("🔐 Generated API token: {}...", &api_token[..16]);

        // Create app state
        let state = AppState {
            api_token: api_token.clone(),
        };

        // Configure CORS to allow webview requests
        // Webviews load from wry://localhost origin
        let cors = CorsLayer::new()
            .allow_origin(Any)  // Allow all origins including wry://localhost
            .allow_methods(Any)
            .allow_headers(Any)
            .expose_headers(Any);  // Allow JavaScript to read response headers

        // Build router with all endpoints
        let app = Router::new()
            .route("/api/command", post(handle_api_request))
            // VFS file serving endpoint for webview subprocess to fetch VFS files
            // Pattern: /vfs/{vfs_id}/pages/{page_id}/{file_path}
            .route("/vfs/:vfs_id/pages/:page_id/*file_path", get(handle_vfs_file))
            .with_state(state)
            .layer(cors);

        // Bind to localhost on random port
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;
        let port = addr.port();

        info!("🚀 API server listening on http://127.0.0.1:{}", port);

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        // Spawn server task
        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    shutdown_rx.await.ok();
                })
                .await
                .expect("API server failed");
        });

        Ok(Arc::new(RwLock::new(Self {
            api_token,
            port,
            shutdown_tx: Some(shutdown_tx),
        })))
    }

    /// Get the API token for passing to webview
    pub fn token(&self) -> &str {
        &self.api_token
    }

    /// Get the server port
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Get the base URL for the API
    pub fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Shutdown the server
    pub fn shutdown(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
            info!("🛑 API server shutdown initiated");
        }
    }
}

/// Generate a random API token (32 bytes = 64 hex characters)
fn generate_api_token() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    hex::encode(bytes)
}

/// Validate API token from request headers
fn validate_token(headers: &HeaderMap, expected_token: &str) -> bool {
    headers
        .get("X-API-Token")
        .and_then(|v| v.to_str().ok())
        .map(|token| token == expected_token)
        .unwrap_or(false)
}

/// Handle API request from webview
///
/// This is the main endpoint that executes commands using main process infrastructure
async fn handle_api_request(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ApiRequest>,
) -> Response {
    // Log request origin for debugging
    if let Some(origin) = headers.get("Origin") {
        info!("📨 API request from origin: {:?}", origin);
    } else {
        info!("📨 API request with no Origin header");
    }

    // Validate API token
    if !validate_token(&headers, &state.api_token) {
        warn!("⚠️ Unauthorized API request: invalid token");
        return (StatusCode::FORBIDDEN, "Invalid API token").into_response();
    }

    info!("📨 API command: {}", request.cmd);

    // Execute command using main process infrastructure
    let result = execute_command(&request.cmd, request.payload).await;

    match result {
        Ok(data) => {
            info!("✅ API response: {} (success)", request.cmd);
            Json(ApiResponse {
                success: true,
                data,
            })
            .into_response()
        }
        Err(e) => {
            warn!("❌ API error: {} - {}", request.cmd, e);
            Json(ApiResponse {
                success: false,
                data: serde_json::json!({ "error": e.to_string() }),
            })
            .into_response()
        }
    }
}

/// Execute command using main process's AWS client and cache
async fn execute_command(
    cmd: &str,
    payload: serde_json::Value,
) -> anyhow::Result<serde_json::Value> {
    use crate::app::webview::commands::*;
    use crate::app::agent_framework::v8_bindings::bindings::{
        resources, cloudwatch_logs, cloudtrail_events,
    };

    match cmd {
        // ========== Core Commands ==========
        "openPage" => {
            let args: OpenPageArgs = serde_json::from_value(payload)?;
            let result = open_page(args).await?;
            Ok(result)
        }

        "logToPageFile" => {
            #[derive(serde::Deserialize)]
            struct LogPayload {
                #[serde(rename = "pageName")]
                page_name: String,
                message: String,
            }

            let args: LogPayload = serde_json::from_value(payload)?;

            // Write to page.log in the page's workspace
            let log_path = dirs::data_local_dir()
                .ok_or_else(|| anyhow::anyhow!("Could not find local data directory"))?
                .join("awsdash/pages")
                .join(&args.page_name)
                .join("page.log");

            // Append to log file
            use std::io::Write;
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)?;

            writeln!(file, "{}", args.message)?;

            Ok(serde_json::json!({
                "status": "success",
                "logPath": log_path.display().to_string()
            }))
        }

        // ========== Direct V8 Binding Commands ==========

        "listAccounts" => {
            list_accounts().await
        }

        "listRegions" => {
            list_regions().await
        }

        "loadCache" => {
            let args: resources::LoadCacheArgs = serde_json::from_value(payload)?;
            load_cache(args).await
        }

        "getResourceSchema" => {
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct SchemaArgs {
                resource_type: String,
            }
            let args: SchemaArgs = serde_json::from_value(payload)?;
            get_resource_schema(args.resource_type).await
        }

        "showInExplorer" => {
            let args: resources::ShowInExplorerArgs = serde_json::from_value(payload)?;
            show_in_explorer(args).await
        }

        "queryCachedResources" => {
            let args: resources::QueryCachedResourcesArgs = serde_json::from_value(payload)?;
            query_cached_resources(args).await
        }

        "listBookmarks" => {
            list_bookmarks().await
        }

        "queryBookmarks" => {
            let args: QueryBookmarksArgs = serde_json::from_value(payload)?;
            query_bookmarks(args).await
        }

        "queryCloudWatchLogEvents" => {
            let args: cloudwatch_logs::QueryCloudWatchLogEventsArgs = serde_json::from_value(payload)?;
            query_cloudwatch_log_events(args).await
        }

        "getCloudTrailEvents" => {
            let args: cloudtrail_events::GetCloudTrailEventsArgs = serde_json::from_value(payload)?;
            get_cloudtrail_events(args).await
        }

        // ========== Page Management Commands ==========

        "listPages" => {
            list_pages().await
        }

        "deletePage" => {
            let args: DeletePageArgs = serde_json::from_value(payload)?;
            delete_page(args).await
        }

        "viewPage" => {
            let args: ViewPageArgs = serde_json::from_value(payload)?;
            view_page(args).await
        }

        "editPage" => {
            let args: EditPageArgs = serde_json::from_value(payload)?;
            edit_page(args).await
        }

        "renamePage" => {
            let args: RenamePageArgs = serde_json::from_value(payload)?;
            rename_page(args).await
        }

        _ => Err(anyhow::anyhow!("Unknown command: {}", cmd)),
    }
}

/// Handle VFS file requests from webview subprocess
///
/// This endpoint allows the webview subprocess (which can't access main process VFS)
/// to fetch VFS files via HTTP. The subprocess custom protocol handler proxies
/// VFS URLs to this endpoint.
///
/// Route: GET /vfs/{vfs_id}/pages/{page_id}/{file_path}
async fn handle_vfs_file(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((vfs_id, page_id, file_path)): Path<(String, String, String)>,
) -> Response {
    // Validate API token
    if !validate_token(&headers, &state.api_token) {
        warn!("⚠️ Unauthorized VFS request: invalid token");
        return (StatusCode::FORBIDDEN, "Invalid API token").into_response();
    }

    let vfs_path = format!("/pages/{}/{}", page_id, file_path);
    info!("📂 VFS file request: vfs_id={}, path={}", vfs_id, vfs_path);

    // Read from VFS registry
    use crate::app::agent_framework::vfs::registry::with_vfs;

    let content = with_vfs(&vfs_id, |vfs| vfs.read_file(&vfs_path).map(|c| c.to_vec()));

    match content {
        Some(Ok(bytes)) => {
            // Determine content type from file extension
            let content_type = match file_path.rsplit('.').next() {
                Some("html") => "text/html",
                Some("js") => "application/javascript",
                Some("css") => "text/css",
                Some("json") => "application/json",
                Some("png") => "image/png",
                Some("jpg") | Some("jpeg") => "image/jpeg",
                Some("gif") => "image/gif",
                Some("svg") => "image/svg+xml",
                _ => "application/octet-stream",
            };

            info!(
                "✅ VFS file served: {} ({} bytes, type: {})",
                vfs_path,
                bytes.len(),
                content_type
            );

            Response::builder()
                .status(StatusCode::OK)
                .header("Content-Type", content_type)
                .header("Access-Control-Allow-Origin", "*")
                .body(axum::body::Body::from(bytes))
                .unwrap()
        }
        Some(Err(e)) => {
            warn!("❌ VFS file not found: {} - {}", vfs_path, e);
            (StatusCode::NOT_FOUND, format!("File not found: {}", vfs_path)).into_response()
        }
        None => {
            warn!("❌ VFS not found: {}", vfs_id);
            (
                StatusCode::NOT_FOUND,
                format!("VFS not found: {}. Agent session may have ended.", vfs_id),
            )
                .into_response()
        }
    }
}
