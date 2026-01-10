#![warn(clippy::all, rust_2018_idioms)]

//! Agent Creation Request/Response System
//!
//! This module provides a channel-based system for tools (like start-task) to request
//! agent creation from AgentManagerWindow without requiring direct window access.

use super::types::AgentId;
use std::collections::HashMap;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, OnceLock};

/// Type alias for the agent creation channel
type AgentCreationChannelType = (
    Sender<AgentCreationRequest>,
    Arc<Mutex<Receiver<AgentCreationRequest>>>,
);

/// Type alias for the agent creation response map
type AgentCreationResponseMap = Arc<Mutex<HashMap<u64, Sender<AgentCreationResponse>>>>;

/// Global agent creation request/response channel
static AGENT_CREATION_CHANNEL: OnceLock<AgentCreationChannelType> = OnceLock::new();

/// Global agent creation response channels
/// Uses a HashMap: each request gets a unique response channel
static AGENT_CREATION_RESPONSES: OnceLock<AgentCreationResponseMap> = OnceLock::new();

/// Counter for generating unique request IDs
static REQUEST_ID_COUNTER: OnceLock<Arc<Mutex<u64>>> = OnceLock::new();

/// Initialize the global agent creation channel
pub fn init_agent_creation_channel() {
    AGENT_CREATION_CHANNEL.get_or_init(|| {
        let (sender, receiver) = channel();
        (sender, Arc::new(Mutex::new(receiver)))
    });

    AGENT_CREATION_RESPONSES.get_or_init(|| Arc::new(Mutex::new(HashMap::new())));

    REQUEST_ID_COUNTER.get_or_init(|| Arc::new(Mutex::new(0)));
}

/// Get the agent creation request sender
pub fn get_agent_creation_sender() -> Sender<AgentCreationRequest> {
    AGENT_CREATION_CHANNEL
        .get_or_init(|| {
            let (sender, receiver) = channel();
            (sender, Arc::new(Mutex::new(receiver)))
        })
        .0
        .clone()
}

/// Get the agent creation request receiver
pub fn get_agent_creation_receiver() -> Arc<Mutex<Receiver<AgentCreationRequest>>> {
    AGENT_CREATION_CHANNEL
        .get_or_init(|| {
            let (sender, receiver) = channel();
            (sender, Arc::new(Mutex::new(receiver)))
        })
        .1
        .clone()
}

/// Generate a unique request ID
fn next_request_id() -> u64 {
    let counter = REQUEST_ID_COUNTER.get_or_init(|| Arc::new(Mutex::new(0)));
    let mut id = counter.lock().unwrap();
    *id += 1;
    *id
}

/// Register a response channel for a specific request
fn register_response_channel(request_id: u64, sender: Sender<AgentCreationResponse>) {
    let responses = AGENT_CREATION_RESPONSES.get_or_init(|| Arc::new(Mutex::new(HashMap::new())));
    responses.lock().unwrap().insert(request_id, sender);
}

/// Get and remove a response channel for a specific request
pub fn take_response_channel(request_id: u64) -> Option<Sender<AgentCreationResponse>> {
    let responses = AGENT_CREATION_RESPONSES.get_or_init(|| Arc::new(Mutex::new(HashMap::new())));
    responses.lock().unwrap().remove(&request_id)
}

/// Request to create a new agent
#[derive(Debug, Clone)]
pub enum AgentCreationRequest {
    /// Request to create a TaskWorker agent
    TaskWorker {
        /// Unique ID for this request (for matching response)
        request_id: u64,
        /// Short description for UI display (3-5 words)
        short_description: String,
        /// Task description to send to the worker
        task_description: String,
        /// Optional expected output format
        expected_output_format: Option<String>,
        /// Parent agent ID (the task-manager spawning this worker)
        parent_id: AgentId,
    },
    /// Request to create a ToolBuilderWorker agent
    ToolBuilderWorker {
        /// Unique ID for this request (for matching response)
        request_id: u64,
        /// Sanitized workspace name (kebab-case, collision-checked)
        workspace_name: String,
        /// Concise description (4-5 words) for inline progress display
        concise_description: String,
        /// Task description to send to the worker
        task_description: String,
        /// Optional resource context
        resource_context: Option<String>,
        /// Parent agent ID (the task-manager spawning this worker)
        parent_id: AgentId,
    },
}

impl AgentCreationRequest {
    /// Get the request ID regardless of variant
    pub fn request_id(&self) -> u64 {
        match self {
            AgentCreationRequest::TaskWorker { request_id, .. } => *request_id,
            AgentCreationRequest::ToolBuilderWorker { request_id, .. } => *request_id,
        }
    }

    /// Get the parent agent ID regardless of variant
    pub fn parent_id(&self) -> AgentId {
        match self {
            AgentCreationRequest::TaskWorker { parent_id, .. } => *parent_id,
            AgentCreationRequest::ToolBuilderWorker { parent_id, .. } => *parent_id,
        }
    }

    /// Get the short description (TaskWorker only)
    pub fn short_description(&self) -> Option<&str> {
        match self {
            AgentCreationRequest::TaskWorker { short_description, .. } => Some(short_description),
            AgentCreationRequest::ToolBuilderWorker { .. } => None,
        }
    }

    /// Get the task description
    pub fn task_description(&self) -> &str {
        match self {
            AgentCreationRequest::TaskWorker { task_description, .. } => task_description,
            AgentCreationRequest::ToolBuilderWorker { task_description, .. } => task_description,
        }
    }

    /// Get the expected output format (TaskWorker only)
    pub fn expected_output_format(&self) -> Option<&str> {
        match self {
            AgentCreationRequest::TaskWorker { expected_output_format, .. } => {
                expected_output_format.as_deref()
            }
            AgentCreationRequest::ToolBuilderWorker { .. } => None,
        }
    }

    /// Get the workspace name (ToolBuilderWorker only)
    pub fn workspace_name(&self) -> Option<&str> {
        match self {
            AgentCreationRequest::TaskWorker { .. } => None,
            AgentCreationRequest::ToolBuilderWorker { workspace_name, .. } => Some(workspace_name),
        }
    }

    /// Get the resource context (ToolBuilderWorker only)
    pub fn resource_context(&self) -> Option<&str> {
        match self {
            AgentCreationRequest::TaskWorker { .. } => None,
            AgentCreationRequest::ToolBuilderWorker { resource_context, .. } => {
                resource_context.as_deref()
            }
        }
    }

    /// Get the concise description (ToolBuilderWorker only)
    pub fn concise_description(&self) -> Option<&str> {
        match self {
            AgentCreationRequest::TaskWorker { .. } => None,
            AgentCreationRequest::ToolBuilderWorker { concise_description, .. } => {
                Some(concise_description)
            }
        }
    }
}

/// Response to agent creation request
#[derive(Debug, Clone)]
pub struct AgentCreationResponse {
    /// The ID of the newly created agent
    pub agent_id: AgentId,

    /// Success or error message
    pub success: bool,

    /// Optional error message
    pub error: Option<String>,

    /// For ToolBuilderWorker: sanitized workspace name
    pub workspace_name: Option<String>,
}

impl AgentCreationRequest {
    /// Create a new TaskWorker agent creation request
    pub fn new(
        short_description: String,
        task_description: String,
        expected_output_format: Option<String>,
        parent_id: AgentId,
    ) -> (Self, Receiver<AgentCreationResponse>) {
        let request_id = next_request_id();

        // Create a one-shot channel for the response
        let (response_sender, response_receiver) = channel();
        register_response_channel(request_id, response_sender);

        let request = Self::TaskWorker {
            request_id,
            short_description,
            task_description,
            expected_output_format,
            parent_id,
        };

        (request, response_receiver)
    }

    /// Create a new ToolBuilderWorker agent creation request
    pub fn new_tool_builder(
        workspace_name: String,
        concise_description: String,
        task_description: String,
        resource_context: Option<String>,
        parent_id: AgentId,
    ) -> (Self, Receiver<AgentCreationResponse>) {
        let request_id = next_request_id();

        // Create a one-shot channel for the response
        let (response_sender, response_receiver) = channel();
        register_response_channel(request_id, response_sender);

        let request = Self::ToolBuilderWorker {
            request_id,
            workspace_name,
            concise_description,
            task_description,
            resource_context,
            parent_id,
        };

        (request, response_receiver)
    }
}

impl AgentCreationResponse {
    /// Create a successful response for TaskWorker
    pub fn success(agent_id: AgentId) -> Self {
        Self {
            agent_id,
            success: true,
            error: None,
            workspace_name: None,
        }
    }

    /// Create a successful response for ToolBuilderWorker
    pub fn success_tool_builder(agent_id: AgentId, workspace_name: String) -> Self {
        Self {
            agent_id,
            success: true,
            error: None,
            workspace_name: Some(workspace_name),
        }
    }

    /// Create an error response
    pub fn error(agent_id: AgentId, error: String) -> Self {
        Self {
            agent_id,
            success: false,
            error: Some(error),
            workspace_name: None,
        }
    }
}

/// Send an agent creation request and wait for response
///
/// This is a convenience function that tools can use to request agent creation.
/// It blocks until the response is received.
pub fn request_agent_creation(
    short_description: String,
    task_description: String,
    expected_output_format: Option<String>,
    parent_id: AgentId,
) -> Result<AgentId, String> {
    stood::perf_checkpoint!(
        "awsdash.request_agent_creation.start",
        &format!("parent_id={}, task={}", parent_id, &short_description)
    );
    let _creation_guard = stood::perf_guard!("awsdash.request_agent_creation");

    let (request, response_receiver) = AgentCreationRequest::new(
        short_description,
        task_description,
        expected_output_format,
        parent_id,
    );

    // Send the request
    stood::perf_checkpoint!(
        "awsdash.request_agent_creation.send_request",
        &format!("request_id={}", request.request_id())
    );
    stood::perf_timed!("awsdash.request_agent_creation.send", {
        get_agent_creation_sender().send(request)
    })
    .map_err(|e| format!("Failed to send agent creation request: {}", e))?;

    // Wait for response (with timeout)
    stood::perf_checkpoint!("awsdash.request_agent_creation.wait_response.start");
    let response = stood::perf_timed!("awsdash.request_agent_creation.recv_timeout", {
        response_receiver.recv_timeout(std::time::Duration::from_secs(5))
    })
    .map_err(|e| format!("Failed to receive agent creation response: {}", e))?;
    stood::perf_checkpoint!(
        "awsdash.request_agent_creation.wait_response.end",
        &format!(
            "agent_id={}, success={}",
            response.agent_id, response.success
        )
    );

    if response.success {
        Ok(response.agent_id)
    } else {
        Err(response
            .error
            .unwrap_or_else(|| "Unknown error".to_string()))
    }
}

/// Send a tool builder creation request and wait for response
///
/// This function:
/// 1. Sanitizes the workspace name (collision detection) - unless reuse_existing is true
/// 2. Creates the workspace directory (or verifies it exists for reuse_existing)
/// 3. Sends the creation request
/// 4. Waits for response
/// 5. Returns (agent_id, workspace_name)
///
/// # Parameters
/// - `suggested_workspace`: The workspace name (will be sanitized unless reuse_existing is true)
/// - `concise_description`: Short description for UI display
/// - `task_description`: Full task description for the worker
/// - `resource_context`: Optional resource context
/// - `parent_id`: Parent agent ID
/// - `reuse_existing`: If true, use the workspace name as-is (for editing existing pages)
pub fn request_page_builder_creation(
    suggested_workspace: String,
    concise_description: String,
    task_description: String,
    resource_context: Option<String>,
    parent_id: AgentId,
    reuse_existing: bool,
) -> Result<(AgentId, String), String> {
    use crate::app::agent_framework::utils::sanitize_workspace_name;

    stood::perf_checkpoint!(
        "awsdash.request_page_builder_creation.start",
        &format!(
            "parent_id={}, workspace={}, reuse_existing={}",
            parent_id, &suggested_workspace, reuse_existing
        )
    );
    let _creation_guard = stood::perf_guard!("awsdash.request_page_builder_creation");

    // Get workspace name - either sanitize for new, or use as-is for existing
    let workspace_name = if reuse_existing {
        // For editing existing pages, use the name directly (already validated by caller)
        stood::perf_checkpoint!(
            "awsdash.request_page_builder_creation.reuse_existing",
            &format!("workspace={}", &suggested_workspace)
        );
        suggested_workspace.clone()
    } else {
        // Sanitize workspace name with collision detection for new pages
        stood::perf_checkpoint!("awsdash.request_page_builder_creation.sanitize_workspace");
        stood::perf_timed!("awsdash.request_page_builder_creation.sanitize", {
            sanitize_workspace_name(&suggested_workspace)
        })
        .map_err(|e| format!("Failed to sanitize workspace name: {}", e))?
    };

    // Workspace directory handling
    stood::perf_checkpoint!(
        "awsdash.request_page_builder_creation.workspace_dir",
        &format!("workspace={}, reuse_existing={}", workspace_name, reuse_existing)
    );
    let workspace_path = dirs::data_local_dir()
        .ok_or_else(|| "Failed to get local data directory".to_string())?
        .join("awsdash/pages")
        .join(&workspace_name);

    if reuse_existing {
        // Verify the workspace exists when reusing
        if !workspace_path.exists() {
            return Err(format!(
                "Cannot edit page '{}': workspace directory not found",
                workspace_name
            ));
        }
    } else {
        // Create workspace directory for new pages
        std::fs::create_dir_all(&workspace_path)
            .map_err(|e| format!("Failed to create workspace directory: {}", e))?;
    }

    // Create request
    let (request, response_receiver) = AgentCreationRequest::new_tool_builder(
        workspace_name.clone(),
        concise_description,
        task_description,
        resource_context,
        parent_id,
    );

    // Send the request
    stood::perf_checkpoint!(
        "awsdash.request_page_builder_creation.send_request",
        &format!("request_id={}", request.request_id())
    );
    stood::perf_timed!("awsdash.request_page_builder_creation.send", {
        get_agent_creation_sender().send(request)
    })
    .map_err(|e| format!("Failed to send tool builder creation request: {}", e))?;

    // Wait for response (with timeout)
    stood::perf_checkpoint!("awsdash.request_page_builder_creation.wait_response.start");
    let response = stood::perf_timed!(
        "awsdash.request_page_builder_creation.recv_timeout",
        response_receiver.recv_timeout(std::time::Duration::from_secs(5))
    )
    .map_err(|e| format!("Failed to receive tool builder creation response: {}", e))?;
    stood::perf_checkpoint!(
        "awsdash.request_page_builder_creation.wait_response.end",
        &format!(
            "agent_id={}, success={}, workspace={}",
            response.agent_id,
            response.success,
            response
                .workspace_name
                .as_ref()
                .unwrap_or(&"<none>".to_string())
        )
    );

    if response.success {
        // Return both agent_id and workspace_name
        let final_workspace = response
            .workspace_name
            .clone()
            .unwrap_or_else(|| workspace_name.clone());
        Ok((response.agent_id, final_workspace))
    } else {
        Err(response
            .error
            .unwrap_or_else(|| "Unknown error".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_creation_request_creation() {
        let parent_id = AgentId::new();
        let (request, _receiver) = AgentCreationRequest::new(
            "Listing instances".to_string(),
            "Test task".to_string(),
            Some("JSON".to_string()),
            parent_id,
        );

        assert!(request.request_id() > 0);
        assert_eq!(request.short_description(), Some("Listing instances"));
        assert_eq!(request.task_description(), "Test task");
        assert_eq!(request.expected_output_format(), Some("JSON"));
        assert_eq!(request.parent_id(), parent_id);
    }

    #[test]
    fn test_agent_creation_response_success() {
        let agent_id = AgentId::new();
        let response = AgentCreationResponse::success(agent_id);

        assert!(response.success);
        assert_eq!(response.agent_id, agent_id);
        assert!(response.error.is_none());
    }

    #[test]
    fn test_agent_creation_response_error() {
        let agent_id = AgentId::new();
        let response = AgentCreationResponse::error(agent_id, "Test error".to_string());

        assert!(!response.success);
        assert_eq!(response.agent_id, agent_id);
        assert_eq!(response.error, Some("Test error".to_string()));
    }

    #[test]
    fn test_channel_initialization() {
        let sender = get_agent_creation_sender();
        let receiver = get_agent_creation_receiver();

        let parent_id = AgentId::new();
        let (request, _resp_receiver) =
            AgentCreationRequest::new("Testing".to_string(), "Test".to_string(), None, parent_id);

        sender.send(request.clone()).unwrap();
        let received = receiver.lock().unwrap().try_recv().unwrap();

        assert_eq!(received.request_id(), request.request_id());
        assert_eq!(received.short_description(), request.short_description());
        assert_eq!(received.task_description(), request.task_description());
    }
}
