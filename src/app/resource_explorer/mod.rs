use crate::app::aws_identity::AwsIdentityCenter;
use egui::Context;
use std::sync::{Arc, Mutex, RwLock as StdRwLock};
use tokio::sync::RwLock;
use tracing::warn;

use bookmarks::BookmarkManager;

/// Actions that the Resource Explorer can request from the main application
#[derive(Debug, Clone)]
pub enum ResourceExplorerAction {
    /// Request to open CloudWatch Logs for a resource
    OpenCloudWatchLogs {
        log_group_name: String,
        resource_name: String,
        account_id: String,
        region: String,
    },
    /// Request to open CloudTrail Events for a resource
    OpenCloudTrailEvents {
        resource_type: String,
        resource_name: String,
        resource_arn: Option<String>,
        account_id: String,
        region: String,
    },
}

// ============================================================================
// Global State for Unified Querying (Agent <-> Explorer Bridge)
// ============================================================================

/// Global access to ResourceExplorerState for unified caching between Agent and Explorer
/// This uses tokio's async RwLock since the state is accessed from async contexts
static GLOBAL_EXPLORER_STATE: StdRwLock<Option<Arc<RwLock<state::ResourceExplorerState>>>> =
    StdRwLock::new(None);

/// Global access to BookmarkManager for V8 bindings
/// This uses std's sync RwLock since V8 callbacks are synchronous
static GLOBAL_BOOKMARK_MANAGER: StdRwLock<Option<Arc<StdRwLock<BookmarkManager>>>> =
    StdRwLock::new(None);

/// Set the global ResourceExplorerState for unified caching (called on login)
pub fn set_global_explorer_state(state: Option<Arc<RwLock<state::ResourceExplorerState>>>) {
    match GLOBAL_EXPLORER_STATE.write() {
        Ok(mut guard) => {
            *guard = state;
        }
        Err(e) => {
            warn!(
                "Failed to update global ResourceExplorerState for V8 bindings: {}",
                e
            );
        }
    }
}

/// Get the global ResourceExplorerState for unified caching
pub fn get_global_explorer_state() -> Option<Arc<RwLock<state::ResourceExplorerState>>> {
    match GLOBAL_EXPLORER_STATE.read() {
        Ok(guard) => guard.clone(),
        Err(e) => {
            warn!(
                "Failed to read global ResourceExplorerState for V8 bindings: {}",
                e
            );
            None
        }
    }
}

/// Set the global BookmarkManager for V8 bindings (called on login)
pub fn set_global_bookmark_manager(manager: Option<Arc<StdRwLock<BookmarkManager>>>) {
    match GLOBAL_BOOKMARK_MANAGER.write() {
        Ok(mut guard) => {
            *guard = manager;
        }
        Err(e) => {
            warn!(
                "Failed to update global BookmarkManager for V8 bindings: {}",
                e
            );
        }
    }
}

/// Get the global BookmarkManager for V8 bindings
pub fn get_global_bookmark_manager() -> Option<Arc<StdRwLock<BookmarkManager>>> {
    match GLOBAL_BOOKMARK_MANAGER.read() {
        Ok(guard) => guard.clone(),
        Err(e) => {
            warn!(
                "Failed to read global BookmarkManager for V8 bindings: {}",
                e
            );
            None
        }
    }
}

pub mod aws_client;
pub mod aws_services;
pub mod bookmarks;
pub mod child_resources;
pub mod colors;
pub mod credentials;
pub mod dialogs;
pub mod global_services;
pub mod normalizers;
pub mod property_system;
pub mod state;
pub mod status;
pub mod tag_badges;
pub mod tag_cache;
pub mod tag_discovery;
pub mod tree;
pub mod unified_query;
pub mod widgets;
pub mod window;

// Verification modules (DEBUG builds only)
#[cfg(debug_assertions)]
pub mod cli_commands;
#[cfg(debug_assertions)]
pub mod verification_results;
#[cfg(debug_assertions)]
pub mod verification_window;

pub use aws_client::{AWSResourceClient, QueryProgress, QueryStatus};
pub use child_resources::{ChildQueryMethod, ChildResourceConfig, ChildResourceDef};
pub use colors::{
    assign_account_color, assign_region_color, get_contrasting_text_color, AwsColorGenerator,
    ColorCacheStats,
};
pub use credentials::{AccountCredentials, CredentialCacheStats, CredentialCoordinator};
pub use dialogs::FuzzySearchDialog;
pub use global_services::{get_global_query_region, is_global_service, GlobalServiceRegistry};
pub use normalizers::NormalizerFactory;
pub use property_system::{
    PropertyCatalog, PropertyFilter, PropertyFilterGroup, PropertyFilterType, PropertyKey,
    PropertyType, PropertyValue,
};
pub use state::{
    AccountSelection, BooleanOperator, GroupingMode, QueryScope, RegionSelection, RelationshipType,
    ResourceEntry, ResourceExplorerState, ResourceRelationship, ResourceTag, ResourceTypeSelection,
    TagFilter, TagFilterGroup, TagFilterType,
};
pub use status::{global_status, report_status, report_status_done, StatusChannel, StatusMessage};
pub use tag_badges::{BadgeSelector, TagCombination, TagPopularityTracker};
pub use tag_cache::{CacheStats, TagCache};
pub use tag_discovery::{OverallTagStats, TagDiscovery, TagMetadata, TagStats};
pub use tree::{NodeType, TreeBuilder, TreeNode, TreeRenderer};
pub use unified_query::{
    BookmarkInfo, DetailLevel, DetailedResources, QueryError, QueryResultStatus, QueryWarning,
    ResourceFull, ResourceSummary, ResourceWithTags, UnifiedQueryResult,
};
pub use window::ResourceExplorerWindow;

/// Main resource explorer interface
pub struct ResourceExplorer {
    #[allow(dead_code)]
    state: Arc<RwLock<ResourceExplorerState>>,
    window: ResourceExplorerWindow,
    pending_actions: Arc<Mutex<Vec<ResourceExplorerAction>>>,
}

impl Default for ResourceExplorer {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceExplorer {
    pub fn new() -> Self {
        let state = Arc::new(RwLock::new(ResourceExplorerState::new()));
        let pending_actions = Arc::new(Mutex::new(Vec::new()));
        let window = ResourceExplorerWindow::new(state.clone(), pending_actions.clone());

        Self {
            state,
            window,
            pending_actions,
        }
    }

    pub fn show(&mut self, ctx: &Context) -> bool {
        self.window.show(ctx)
    }

    pub fn is_open(&self) -> bool {
        self.window.is_open()
    }

    pub fn set_open(&mut self, open: bool) {
        self.window.set_open(open);
    }

    /// Set the AWS Identity Center reference to access real account data
    pub fn set_aws_identity_center(
        &mut self,
        aws_identity_center: Option<Arc<Mutex<AwsIdentityCenter>>>,
    ) {
        self.window.set_aws_identity_center(aws_identity_center);
    }

    /// Get reference to the AWS client for use by other components
    pub fn get_aws_client(&self) -> Option<Arc<AWSResourceClient>> {
        self.window.get_aws_client()
    }

    /// Get and clear pending actions from the Resource Explorer
    pub fn take_pending_actions(&mut self) -> Vec<ResourceExplorerAction> {
        if let Ok(mut actions) = self.pending_actions.lock() {
            std::mem::take(&mut *actions)
        } else {
            Vec::new()
        }
    }

    /// Get the ResourceExplorerState for unified caching with V8 bindings
    pub fn get_state(&self) -> Arc<RwLock<state::ResourceExplorerState>> {
        self.window.get_state()
    }

    /// Get the BookmarkManager for unified access with V8 bindings
    pub fn get_bookmark_manager(&self) -> Arc<StdRwLock<BookmarkManager>> {
        self.window.get_bookmark_manager()
    }
}
