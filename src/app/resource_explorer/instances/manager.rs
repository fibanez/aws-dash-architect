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
pub struct ExplorerSharedContext {
    /// Shared Moka cache (global singleton)
    pub cache: Arc<SharedResourceCache>,
    /// Global bookmarks (shared across all panes)
    pub bookmarks: Arc<StdRwLock<BookmarkManager>>,
    /// AWS client for making API calls
    pub aws_client: Option<Arc<AWSResourceClient>>,
    /// AWS Identity Center for account access
    pub aws_identity_center: Option<Arc<Mutex<AwsIdentityCenter>>>,
}

impl ExplorerSharedContext {
    /// Create a new shared context
    pub fn new() -> Self {
        // Create bookmark manager - panic if we can't access config directory
        let bookmark_manager = BookmarkManager::new()
            .expect("Failed to initialize bookmark manager");

        Self {
            cache: crate::app::resource_explorer::cache::shared_cache(),
            bookmarks: Arc::new(StdRwLock::new(bookmark_manager)),
            aws_client: None,
            aws_identity_center: None,
        }
    }

    /// Set the AWS client
    pub fn set_aws_client(&mut self, client: Option<Arc<AWSResourceClient>>) {
        self.aws_client = client;
    }

    /// Set the AWS Identity Center
    pub fn set_aws_identity_center(&mut self, identity_center: Option<Arc<Mutex<AwsIdentityCenter>>>) {
        self.aws_identity_center = identity_center;
    }

    /// Get the AWS client
    pub fn get_aws_client(&self) -> Option<Arc<AWSResourceClient>> {
        self.aws_client.clone()
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
    /// Create a new manager with one default instance
    pub fn new() -> Self {
        let mut manager = Self {
            shared_context: ExplorerSharedContext::new(),
            instances: Vec::new(),
            focused_instance_id: None,
            next_instance_number: 1,
        };

        // Create the first instance
        let first_instance_id = manager.create_instance();
        manager.focused_instance_id = Some(first_instance_id);

        manager
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
    pub fn set_aws_identity_center(&mut self, identity_center: Option<Arc<Mutex<AwsIdentityCenter>>>) {
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
}
