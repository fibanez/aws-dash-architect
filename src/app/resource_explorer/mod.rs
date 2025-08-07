use crate::app::aws_identity::AwsIdentityCenter;
use egui::Context;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

pub mod aws_client;
pub mod aws_services;
pub mod colors;
pub mod credentials;
pub mod dialogs;
pub mod global_services;
pub mod normalizers;
pub mod state;
pub mod tree;
pub mod window;

pub use aws_client::{AWSResourceClient, QueryProgress, QueryStatus};
pub use colors::{
    assign_account_color, assign_region_color, get_contrasting_text_color, AwsColorGenerator,
    ColorCacheStats,
};
pub use credentials::{AccountCredentials, CredentialCacheStats, CredentialCoordinator};
pub use dialogs::FuzzySearchDialog;
pub use global_services::{is_global_service, get_global_query_region, GlobalServiceRegistry};
pub use normalizers::{NormalizerFactory, ResourceNormalizer};
pub use state::{
    AccountSelection, GroupingMode, QueryScope, RegionSelection, RelationshipType, ResourceEntry,
    ResourceExplorerState, ResourceRelationship, ResourceTag, ResourceTypeSelection,
};
pub use tree::{NodeType, TreeBuilder, TreeNode, TreeRenderer};
pub use window::ResourceExplorerWindow;

/// Main resource explorer interface
pub struct ResourceExplorer {
    #[allow(dead_code)]
    state: Arc<RwLock<ResourceExplorerState>>,
    window: ResourceExplorerWindow,
}

impl Default for ResourceExplorer {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceExplorer {
    pub fn new() -> Self {
        let state = Arc::new(RwLock::new(ResourceExplorerState::new()));
        let window = ResourceExplorerWindow::new(state.clone());

        Self { state, window }
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
}
