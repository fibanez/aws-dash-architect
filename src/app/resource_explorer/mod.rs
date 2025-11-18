use crate::app::aws_identity::AwsIdentityCenter;
use egui::Context;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

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
pub mod tag_badges;
pub mod tag_cache;
pub mod tag_discovery;
pub mod tree;
pub mod widgets;
pub mod window;

pub use aws_client::{AWSResourceClient, QueryProgress, QueryStatus};
pub use child_resources::{ChildQueryMethod, ChildResourceConfig, ChildResourceDef};
pub use colors::{
    assign_account_color, assign_region_color, get_contrasting_text_color, AwsColorGenerator,
    ColorCacheStats,
};
pub use credentials::{AccountCredentials, CredentialCacheStats, CredentialCoordinator};
pub use dialogs::FuzzySearchDialog;
pub use global_services::{is_global_service, get_global_query_region, GlobalServiceRegistry};
pub use normalizers::NormalizerFactory;
pub use property_system::{
    PropertyCatalog, PropertyFilter, PropertyFilterGroup, PropertyFilterType, PropertyKey,
    PropertyType, PropertyValue,
};
pub use state::{
    AccountSelection, BooleanOperator, GroupingMode, QueryScope, RegionSelection, RelationshipType,
    ResourceEntry, ResourceExplorerState, ResourceRelationship, ResourceTag,
    ResourceTypeSelection, TagFilter, TagFilterGroup, TagFilterType,
};
pub use tag_badges::{BadgeSelector, TagCombination, TagPopularityTracker};
pub use tag_cache::{CacheStats, TagCache};
pub use tag_discovery::{OverallTagStats, TagDiscovery, TagMetadata, TagStats};
pub use tree::{NodeType, TreeBuilder, TreeNode, TreeRenderer};
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

        Self { state, window, pending_actions }
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
}
