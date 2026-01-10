//! Explorer Manager - Manages all Explorer windows and shared context
//!
//! The ExplorerManager is responsible for:
//! - Managing multiple ExplorerInstance windows
//! - Providing shared context (cache, bookmarks, AWS client) to all panes
//! - Tracking which window is focused

use super::instance::ExplorerInstance;
use crate::app::aws_identity::AwsIdentityCenter;
use crate::app::resource_explorer::bookmarks::BookmarkManager;
use crate::app::resource_explorer::cache::SharedResourceCache;
use crate::app::resource_explorer::AWSResourceClient;
use std::sync::{Arc, Mutex, RwLock as StdRwLock};
use uuid::Uuid;

/// Shared context across all panes (cache, bookmarks, AWS client)
#[derive(Clone)]
pub struct ExplorerSharedContext {
    /// Shared Moka cache (global singleton)
    pub cache: Arc<SharedResourceCache>,
    /// Global bookmarks (shared across all panes)
    pub bookmarks: Arc<StdRwLock<BookmarkManager>>,
    /// AWS client for making API calls
    pub aws_client: Option<Arc<AWSResourceClient>>,
    /// AWS Identity Center for account access
    pub aws_identity_center: Option<Arc<Mutex<AwsIdentityCenter>>>,
    /// Console role menu updates (for async role fetching in context menus)
    pub console_role_menu_updates: Arc<Mutex<Vec<crate::app::resource_explorer::ConsoleRoleMenuUpdate>>>,
    /// Modular query engine (created when AWS client is available)
    pub query_engine: Option<Arc<super::super::ResourceQueryEngine>>,
}

impl ExplorerSharedContext {
    /// Create a new shared context
    pub fn new() -> Self {
        // Create bookmark manager - panic if we can't access config directory
        let bookmark_manager =
            BookmarkManager::new().expect("Failed to initialize bookmark manager");

        Self {
            cache: crate::app::resource_explorer::cache::shared_cache(),
            bookmarks: Arc::new(StdRwLock::new(bookmark_manager)),
            aws_client: None,
            aws_identity_center: None,
            console_role_menu_updates: Arc::new(Mutex::new(Vec::new())),
            query_engine: None,
        }
    }

    /// Set the AWS client and update the query engine
    pub fn set_aws_client(&mut self, client: Option<Arc<AWSResourceClient>>) {
        self.aws_client = client.clone();

        // Update query engine when client changes
        if let Some(aws_client) = client {
            // Create new query engine with updated client
            let engine = super::super::ResourceQueryEngine::new(
                aws_client,
                self.cache.clone(),
            );
            self.query_engine = Some(Arc::new(engine));
            tracing::debug!("Query engine created/updated with new AWS client");
        } else {
            // Clear query engine when client is removed (e.g., on logout)
            self.query_engine = None;
            tracing::debug!("Query engine cleared (no AWS client)");
        }
    }

    /// Set the AWS Identity Center
    pub fn set_aws_identity_center(
        &mut self,
        identity_center: Option<Arc<Mutex<AwsIdentityCenter>>>,
    ) {
        self.aws_identity_center = identity_center;
    }

    /// Get the AWS client
    pub fn get_aws_client(&self) -> Option<Arc<AWSResourceClient>> {
        self.aws_client.clone()
    }

    /// Get the console role menu updates queue
    pub fn console_role_menu_updates(&self) -> Arc<Mutex<Vec<crate::app::resource_explorer::ConsoleRoleMenuUpdate>>> {
        self.console_role_menu_updates.clone()
    }
}

impl Default for ExplorerSharedContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Manager for all Explorer windows
pub struct ExplorerManager {
    /// Shared context for all panes
    pub shared_context: ExplorerSharedContext,
    /// All Explorer instances (windows)
    pub instances: Vec<ExplorerInstance>,
    /// ID of the currently focused instance
    pub focused_instance_id: Option<Uuid>,
    /// Counter for naming new instances
    next_instance_number: usize,
}

impl Default for ExplorerManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ExplorerManager {
    /// Create a new manager with no initial instances
    ///
    /// Instances are created on-demand via menu clicks or programmatic calls
    pub fn new() -> Self {
        Self {
            shared_context: ExplorerSharedContext::new(),
            instances: Vec::new(),
            focused_instance_id: None,
            next_instance_number: 1,
        }
    }

    /// Create a new Explorer instance, add it to the manager, and return its ID
    fn create_instance(&mut self) -> Uuid {
        let instance = ExplorerInstance::new(self.next_instance_number);
        let id = instance.id();
        self.next_instance_number += 1;
        self.instances.push(instance);
        id
    }

    /// Open a new Explorer window
    pub fn open_new_window(&mut self) -> &mut ExplorerInstance {
        let id = self.create_instance();
        self.focused_instance_id = Some(id);
        self.instances.last_mut().unwrap()
    }

    /// Close an Explorer window by ID
    pub fn close_window(&mut self, id: Uuid) {
        if let Some(pos) = self.instances.iter().position(|i| i.id() == id) {
            self.instances.remove(pos);

            // Update focused instance if needed
            if self.focused_instance_id == Some(id) {
                self.focused_instance_id = self.instances.first().map(|i| i.id());
            }
        }
    }

    /// Get the list of all open windows for the Windows menu
    pub fn get_window_list(&self) -> Vec<(Uuid, String, bool)> {
        self.instances
            .iter()
            .map(|i| {
                let is_focused = self.focused_instance_id == Some(i.id());
                (i.id(), i.title.clone(), is_focused)
            })
            .collect()
    }

    /// Focus a specific window
    pub fn focus_window(&mut self, id: Uuid) {
        if self.instances.iter().any(|i| i.id() == id) {
            self.focused_instance_id = Some(id);
        }
    }

    /// Get the currently focused instance
    pub fn focused_instance(&self) -> Option<&ExplorerInstance> {
        self.focused_instance_id
            .and_then(|id| self.instances.iter().find(|i| i.id() == id))
    }

    /// Get the currently focused instance mutably
    pub fn focused_instance_mut(&mut self) -> Option<&mut ExplorerInstance> {
        let focused_id = self.focused_instance_id;
        focused_id.and_then(move |id| self.instances.iter_mut().find(|i| i.id() == id))
    }

    /// Get an instance by ID
    pub fn get_instance(&self, id: Uuid) -> Option<&ExplorerInstance> {
        self.instances.iter().find(|i| i.id() == id)
    }

    /// Get an instance by ID mutably
    pub fn get_instance_mut(&mut self, id: Uuid) -> Option<&mut ExplorerInstance> {
        self.instances.iter_mut().find(|i| i.id() == id)
    }

    /// Get the number of open windows
    pub fn window_count(&self) -> usize {
        self.instances.len()
    }

    /// Check if there are any open windows
    pub fn has_open_windows(&self) -> bool {
        self.instances.iter().any(|i| i.is_open)
    }

    /// Set AWS Identity Center for all instances
    pub fn set_aws_identity_center(
        &mut self,
        identity_center: Option<Arc<Mutex<AwsIdentityCenter>>>,
    ) {
        self.shared_context.set_aws_identity_center(identity_center);
    }

    /// Set AWS client for all instances
    pub fn set_aws_client(&mut self, client: Option<Arc<AWSResourceClient>>) {
        self.shared_context.set_aws_client(client);
    }

    /// Get the bookmarks manager
    pub fn get_bookmark_manager(&self) -> Arc<StdRwLock<BookmarkManager>> {
        self.shared_context.bookmarks.clone()
    }

    /// Close all Explorer windows (called on logout)
    pub fn close_all_windows(&mut self) {
        self.instances.clear();
        self.focused_instance_id = None;

        // Clear global explorer state for V8 bindings
        use crate::app::resource_explorer::set_global_explorer_state;
        set_global_explorer_state(None);
    }

    /// Take pending ResourceExplorerActions from all instances
    ///
    /// Collects actions from all instances (windows), all tabs, and all panes.
    /// These actions are then processed by the main application (e.g., open CloudWatch Logs)
    pub fn take_pending_actions(
        &mut self,
    ) -> Vec<crate::app::resource_explorer::ResourceExplorerAction> {
        let mut actions = Vec::new();

        for instance in &mut self.instances {
            actions.extend(instance.take_pending_actions());
        }

        actions
    }
}
