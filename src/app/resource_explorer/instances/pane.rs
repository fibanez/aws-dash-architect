//! Explorer Pane - A single explorer pane with independent state
//!
//! Each pane has its own ResourceExplorerState and can independently:
//! - Query resources
//! - Apply filters
//! - Navigate the tree view
//! - Apply bookmarks

use super::pane_renderer::{PaneAction, PaneRenderer};
use crate::app::resource_explorer::dialogs::FuzzySearchDialog;
use crate::app::resource_explorer::state::ResourceExplorerState;
use egui::{Context, Ui};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// A single explorer pane with independent state
pub struct ExplorerPane {
    /// Unique identifier for this pane
    pub id: Uuid,
    /// Independent state for this pane (query scope, filters, resources, tree)
    /// Wrapped in Arc<RwLock> for async access from query tasks
    pub state: Arc<RwLock<ResourceExplorerState>>,
    /// Renderer for this pane (tree view, active tags, search, etc.)
    pub renderer: PaneRenderer,
    /// Fuzzy search dialog for account/region/resource type selection
    pub fuzzy_dialog: FuzzySearchDialog,
    /// UI state: scroll offset in the tree view
    pub scroll_offset: f32,
    /// UI state: currently selected resource ARN
    pub selected_resource: Option<String>,

    // Dialog flags (local to avoid borrow conflicts)
    show_refresh_dialog: bool,
    show_bookmark_dialog: bool,
    show_bookmark_manager: bool,
    bookmark_dialog_name: String,
    bookmark_dialog_description: String,
    bookmark_dialog_folder_id: Option<String>,

    // Deferred query trigger (set during rendering, executed after lock released)
    pending_query_trigger: bool,
}

impl Default for ExplorerPane {
    fn default() -> Self {
        Self::new()
    }
}

impl ExplorerPane {
    /// Create a new empty pane
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            state: Arc::new(RwLock::new(ResourceExplorerState::new())),
            renderer: PaneRenderer::new(),
            fuzzy_dialog: FuzzySearchDialog::new(),
            scroll_offset: 0.0,
            selected_resource: None,
            show_refresh_dialog: false,
            show_bookmark_dialog: false,
            show_bookmark_manager: false,
            bookmark_dialog_name: String::new(),
            bookmark_dialog_description: String::new(),
            bookmark_dialog_folder_id: None,
            pending_query_trigger: false,
        }
    }

    /// Get the pane's unique ID
    pub fn id(&self) -> Uuid {
        self.id
    }

    /// Get the state for read access
    pub fn get_state(&self) -> Arc<RwLock<ResourceExplorerState>> {
        self.state.clone()
    }

    /// Get the renderer mutably
    pub fn renderer_mut(&mut self) -> &mut PaneRenderer {
        &mut self.renderer
    }

    /// Check if this pane has any resources loaded
    pub fn has_resources(&self) -> bool {
        self.state
            .try_read()
            .map(|s| !s.resources.is_empty())
            .unwrap_or(false)
    }

    /// Check if this pane is currently loading
    pub fn is_loading(&self) -> bool {
        self.state
            .try_read()
            .map(|s| s.is_loading())
            .unwrap_or(false)
    }

    /// Get the number of resources in this pane
    pub fn resource_count(&self) -> usize {
        self.state
            .try_read()
            .map(|s| s.resources.len())
            .unwrap_or(0)
    }

    /// Clear the pane state (like terminate, but preserves pane identity)
    pub fn clear(&mut self) {
        if let Ok(mut state) = self.state.try_write() {
            *state = ResourceExplorerState::new();
        }
        self.renderer.reset();
        self.fuzzy_dialog = FuzzySearchDialog::new();
        self.scroll_offset = 0.0;
        self.selected_resource = None;
        self.show_refresh_dialog = false;
        self.show_bookmark_dialog = false;
        self.show_bookmark_manager = false;
        self.bookmark_dialog_name.clear();
        self.bookmark_dialog_description.clear();
        self.bookmark_dialog_folder_id = None;
        self.pending_query_trigger = false;
    }

    /// Render the pane content
    ///
    /// This method renders the complete pane UI including:
    /// - Active selection tags
    /// - Search bar
    /// - Tree view of resources
    ///
    /// Returns a list of actions triggered during rendering (e.g., tag removals)
    /// that should be processed by the window/manager.
    pub fn render(
        &mut self,
        ui: &mut Ui,
        shared_context: &super::manager::ExplorerSharedContext,
    ) -> Vec<PaneAction> {
        // Try to acquire write lock on state (non-blocking to avoid UI freeze)
        if let Ok(mut state) = self.state.try_write() {
            // Pass pane ID and shared context to renderer for unique widget IDs and bookmarks access
            self.renderer.render_with_id(ui, &mut state, self.id, shared_context)
        } else {
            // State is locked (probably by async query), show loading indicator
            ui.centered_and_justified(|ui| {
                ui.spinner();
                ui.label("Loading...");
            });
            Vec::new()
        }
    }

    /// Take pending ResourceExplorerActions from the pane renderer
    ///
    /// These are actions like opening CloudWatch Logs, CloudTrail Events, or AWS Console
    /// that should be processed by the main application.
    pub fn take_pending_actions(
        &mut self,
    ) -> Vec<crate::app::resource_explorer::ResourceExplorerAction> {
        self.renderer.take_pending_actions()
    }

    /// Render all dialogs for this pane
    ///
    /// This method should be called from the instance level where Context is available.
    /// It handles all modal dialogs (selection, bookmark, refresh, etc.)
    pub fn render_dialogs(
        &mut self,
        ctx: &Context,
        shared_context: &super::manager::ExplorerSharedContext,
    ) {
        use crate::app::resource_explorer::dialogs::{get_default_regions, get_default_resource_types};

        // Unified selection dialog - handle directly to avoid borrow conflicts
        if let Ok(mut state) = self.state.try_write() {
            if state.show_unified_selection_dialog {
                // Get available accounts from AWS Identity Center
                let available_accounts = if let Some(ref identity_center) = shared_context.aws_identity_center {
                    if let Ok(ic) = identity_center.lock() {
                        ic.accounts.clone() // Clone the Vec<AwsAccount>
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                };

                // Get current selections to pre-populate the dialog
                let current_accounts = state.query_scope.accounts.clone();
                let current_regions = state.query_scope.regions.clone();
                let current_resources = state.query_scope.resource_types.clone();

                // Merge default regions with currently selected regions (from bookmarks)
                let mut available_regions = get_default_regions();
                for region in &current_regions {
                    if !available_regions.contains(&region.region_code) {
                        available_regions.push(region.region_code.clone());
                    }
                }

                // Merge default resource types with currently selected ones (from bookmarks)
                let mut available_resource_types = get_default_resource_types();
                for resource in &current_resources {
                    if !available_resource_types
                        .iter()
                        .any(|rt| rt.resource_type == resource.resource_type)
                    {
                        available_resource_types.push(resource.clone());
                    }
                }

                // Show the dialog and handle selections
                if let Some((accounts, regions, resources)) =
                    self.fuzzy_dialog.show_unified_selection_dialog(
                        ctx,
                        &mut state.show_unified_selection_dialog,
                        &available_accounts,
                        &available_regions,
                        &available_resource_types,
                        &current_accounts,
                        &current_regions,
                        &current_resources,
                    )
                {
                    // Replace current selections with new ones from dialog
                    state.query_scope.accounts.clear();
                    for account in accounts {
                        tracing::info!(
                            "Pane {}: Setting account: {} ({})",
                            self.id,
                            account.display_name,
                            account.account_id
                        );
                        state.add_account(account);
                    }

                    state.query_scope.regions.clear();
                    for region in regions {
                        tracing::info!("Pane {}: Setting region: {} ({})", self.id, region.display_name, region.region_code);
                        state.add_region(region);
                    }

                    state.query_scope.resource_types.clear();
                    for resource in resources {
                        tracing::info!("Pane {}: Setting resource type: {}", self.id, resource.resource_type);
                        state.add_resource_type(resource);
                    }

                    // Drop the state lock before triggering query
                    drop(state);

                    // Set flag to trigger query after rendering completes
                    self.pending_query_trigger = true;
                    tracing::debug!("Pane {}: Marked for pending query trigger", self.id);
                }
            }
        }

        // TODO: Add other dialogs (refresh, bookmark, etc.)
    }

    /// Trigger resource query if selections are ready and not currently loading
    ///
    /// Based on window.rs trigger_query_if_ready (lines 3434-3474)
    fn trigger_query_if_ready(
        &self,
        ctx: &Context,
        shared_context: &super::manager::ExplorerSharedContext,
    ) {
        tracing::info!("Pane {}: trigger_query_if_ready called", self.id);

        // Check if we have selections and not already loading
        if let Ok(state) = self.state.try_read() {
            if state.query_scope.is_empty() {
                tracing::warn!("Pane {}: Query scope is empty, not triggering", self.id);
                return;
            }

            if state.is_loading() {
                tracing::warn!("Pane {}: Already loading, not triggering", self.id);
                return;
            }

            tracing::info!(
                "Pane {}: Triggering query for {} account(s) × {} region(s) × {} resource type(s)",
                self.id,
                state.query_scope.accounts.len(),
                state.query_scope.regions.len(),
                state.query_scope.resource_types.len()
            );
        } else {
            tracing::error!("Pane {}: Failed to acquire state read lock", self.id);
            return;
        }

        // Get query engine from shared context
        let query_engine = match &shared_context.query_engine {
            Some(engine) => engine.clone(),
            None => {
                tracing::warn!("Pane {}: Query engine not available - AWS client may not be configured", self.id);
                return;
            }
        };

        // Clone resources for async operation
        let state_arc = self.state.clone();

        // Get query scope
        let scope = if let Ok(state) = self.state.try_read() {
            state.query_scope.clone()
        } else {
            return;
        };

        // Mark as loading
        let cache_key = if let Ok(mut state) = self.state.try_write() {
            state.start_loading_task(&format!("pane_{}_query", self.id))
        } else {
            format!("pane_{}_fallback_{}", self.id, chrono::Utc::now().timestamp_millis())
        };

        // Request UI repaint
        ctx.request_repaint_after(std::time::Duration::from_millis(50));

        // Execute query via modular UI adapter
        let ui_adapter = super::super::UIQueryAdapter::new(query_engine);
        ui_adapter.execute_for_pane(state_arc, scope, cache_key, ctx.clone());
    }

    /// Mark this pane to trigger a query after rendering completes
    ///
    /// Used by bookmark loading and other operations that need to trigger
    /// queries outside the rendering context to avoid lock conflicts.
    pub fn mark_pending_query(&mut self) {
        self.pending_query_trigger = true;
        tracing::debug!("Pane {}: Marked for pending query trigger", self.id);
    }

    /// Execute any pending query triggers (called after rendering to avoid lock conflicts)
    pub fn execute_pending_query(
        &mut self,
        ctx: &Context,
        shared_context: &super::manager::ExplorerSharedContext,
    ) {
        if self.pending_query_trigger {
            tracing::debug!("Pane {}: Executing pending query trigger", self.id);
            self.pending_query_trigger = false;
            self.trigger_query_if_ready(ctx, shared_context);
        }
    }
}
