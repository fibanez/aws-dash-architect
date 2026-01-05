use super::{
    aws_client::*, bookmarks::*, dialogs::*, instances::pane_renderer::PaneAction,
    instances::pane_renderer::PaneRenderer, retry_tracker::retry_tracker,
    sdk_errors::ErrorCategory, state::*, status::global_status, tree::*, widgets::*,
};
#[cfg(debug_assertions)]
use super::verification_window::VerificationWindow;
use crate::app::agent_framework::tools_registry::set_global_aws_client;
use crate::app::aws_identity::AwsIdentityCenter;
use egui::{Color32, Context, Ui, Window};
use egui_dnd::dnd;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock as StdRwLock};
use tokio::sync::RwLock;
use tracing::warn;

/// Drag-drop payload that supports both bookmarks and folders
#[derive(Clone, serde::Serialize, serde::Deserialize)]
enum DragData {
    Bookmark {
        id: String,
        source_folder: Option<String>,
    },
    Folder {
        id: String,
        parent_id: Option<String>,
    },
}

/// Action requested by the Explorer window
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WindowAction {
    /// No action, window continues normally
    #[default]
    None,
    /// Minimize - hide window but keep state (can be restored)
    Minimize,
    /// Terminate - destroy window and clear state (cache preserved)
    Terminate,
}

pub struct ResourceExplorerWindow {
    state: Arc<RwLock<ResourceExplorerState>>,
    is_open: bool,
    is_focused: bool,
    /// Track if minimize was requested (to distinguish from X button terminate)
    minimize_requested: bool,
    fuzzy_dialog: FuzzySearchDialog,
    tree_renderer: TreeRenderer,
    aws_client: Option<Arc<AWSResourceClient>>,
    refresh_selection: HashMap<String, bool>, // Track which combinations to refresh (display name -> selected)
    refresh_display_to_cache: HashMap<String, String>, // Map display name to cache key
    show_refresh_dialog: bool,                // Local dialog state to avoid borrow conflicts
    show_filter_builder: bool,                // Local filter builder dialog state
    filter_builder_working_group: Option<TagFilterGroup>, // In-progress filter group (persists while dialog is open)
    show_hierarchy_builder: bool,                         // Local hierarchy builder dialog state
    hierarchy_builder_widget: Option<TagHierarchyBuilderWidget>, // Widget instance (persists for state continuity)
    show_property_filter_builder: bool, // Local property filter builder dialog state
    property_filter_builder_working_group:
        Option<crate::app::resource_explorer::PropertyFilterGroup>, // In-progress property filter group
    show_property_hierarchy_builder: bool, // Local property hierarchy builder dialog state
    property_hierarchy_builder_widget:
        Option<crate::app::resource_explorer::widgets::PropertyHierarchyBuilderWidget>, // Widget instance (persists for state continuity)
    aws_identity_center: Option<Arc<Mutex<AwsIdentityCenter>>>, // Access to real AWS accounts
    failed_detail_requests: Arc<tokio::sync::RwLock<std::collections::HashSet<String>>>, // Track failed requests
    frame_count: u64, // Frame counter for debouncing logs

    // Bookmark system - Arc-wrapped for sharing with V8 bindings
    bookmark_manager: Arc<StdRwLock<BookmarkManager>>,
    show_bookmark_dialog: bool,
    show_bookmark_manager: bool,
    bookmark_dialog_name: String,
    bookmark_dialog_description: String,
    bookmark_dialog_folder_id: Option<String>, // Folder to create bookmark in (None = Top Folder)
    show_bookmark_edit_dialog: bool,
    editing_bookmark_id: Option<String>,
    bookmark_edit_name: String,
    bookmark_edit_description: String,

    // Folder management
    show_folder_dialog: bool,
    folder_dialog_name: String,
    folder_dialog_parent_id: Option<String>,
    editing_folder_id: Option<String>,
    expanded_folders: std::collections::HashSet<String>, // Track expanded folders in tree view

    // Copy/paste clipboard
    bookmark_clipboard: Option<String>, // Bookmark ID in clipboard
    bookmark_clipboard_is_cut: bool,    // True if cut operation, false if copy

    // Pending actions to communicate with main app
    pending_actions: Arc<Mutex<Vec<super::ResourceExplorerAction>>>,
    console_role_menu_updates: Arc<Mutex<Vec<super::ConsoleRoleMenuUpdate>>>,

    // Service availability dialog (shows failed queries with error categories)
    show_service_availability_dialog: bool,
    last_failed_queries: std::collections::HashMap<String, ErrorCategory>, // Snapshot of failed queries with error categories for dialog
    last_failed_queries_snapshotted: bool, // Track if we've already snapshotted to prevent per-frame execution

    // Verification window (DEBUG builds only)
    #[cfg(debug_assertions)]
    verification_window: VerificationWindow,
}

impl ResourceExplorerWindow {
    pub fn new(
        state: Arc<RwLock<ResourceExplorerState>>,
        pending_actions: Arc<Mutex<Vec<super::ResourceExplorerAction>>>,
        console_role_menu_updates: Arc<Mutex<Vec<super::ConsoleRoleMenuUpdate>>>,
    ) -> Self {
        Self {
            state,
            is_open: false,
            is_focused: false,
            minimize_requested: false,
            fuzzy_dialog: FuzzySearchDialog::new(),
            tree_renderer: TreeRenderer::new(),
            aws_client: None,
            refresh_selection: HashMap::new(),
            refresh_display_to_cache: HashMap::new(),
            show_refresh_dialog: false,
            show_filter_builder: false,
            filter_builder_working_group: None,
            show_hierarchy_builder: false,
            show_property_filter_builder: false,
            property_filter_builder_working_group: None,
            show_property_hierarchy_builder: false,
            property_hierarchy_builder_widget: None,
            hierarchy_builder_widget: None,
            aws_identity_center: None,
            failed_detail_requests: Arc::new(tokio::sync::RwLock::new(
                std::collections::HashSet::new(),
            )),
            frame_count: 0,
            bookmark_manager: Arc::new(StdRwLock::new(BookmarkManager::new().unwrap_or_default())),
            show_bookmark_dialog: false,
            show_bookmark_manager: false,
            bookmark_dialog_name: String::new(),
            bookmark_dialog_description: String::new(),
            bookmark_dialog_folder_id: None,
            show_bookmark_edit_dialog: false,
            editing_bookmark_id: None,
            bookmark_edit_name: String::new(),
            bookmark_edit_description: String::new(),
            show_folder_dialog: false,
            folder_dialog_name: String::new(),
            folder_dialog_parent_id: None,
            editing_folder_id: None,
            expanded_folders: std::collections::HashSet::new(),
            bookmark_clipboard: None,
            bookmark_clipboard_is_cut: false,
            pending_actions,
            show_service_availability_dialog: false,
            last_failed_queries: std::collections::HashMap::new(),
            last_failed_queries_snapshotted: false,
            console_role_menu_updates,
            #[cfg(debug_assertions)]
            verification_window: VerificationWindow::new(),
        }
    }

    /// Get the ResourceExplorerState for unified caching with V8 bindings
    pub fn get_state(&self) -> Arc<RwLock<ResourceExplorerState>> {
        self.state.clone()
    }

    /// Get the BookmarkManager for unified access with V8 bindings
    pub fn get_bookmark_manager(&self) -> Arc<StdRwLock<BookmarkManager>> {
        self.bookmark_manager.clone()
    }

    /// Get user-friendly error category label for display
    fn error_category_label(category: &ErrorCategory) -> String {
        match category {
            ErrorCategory::NetworkError { message } => {
                if message.is_empty() {
                    "Network Error".to_string()
                } else {
                    format!("Network Error: {}", message)
                }
            }
            ErrorCategory::Throttled { service, .. } => {
                format!("Rate Limited: {}", service)
            }
            ErrorCategory::Timeout { .. } => "Timeout".to_string(),
            ErrorCategory::ServiceUnavailable { service, .. } => {
                format!("Service Unavailable: {}", service)
            }
            ErrorCategory::NonRetryable { code, message, is_permission_error } => {
                if *is_permission_error {
                    format!("Permission Denied: {}", code)
                } else if code == "Error" && !message.is_empty() {
                    // Show the actual error message when code is generic
                    message.clone()
                } else {
                    code.clone()
                }
            }
        }
    }

    /// Get color for error category display
    fn error_category_color(category: &ErrorCategory) -> Color32 {
        match category {
            ErrorCategory::NetworkError { .. } => Color32::from_rgb(255, 150, 50),
            ErrorCategory::Throttled { .. } => Color32::from_rgb(255, 200, 100),
            ErrorCategory::Timeout { .. } => Color32::from_rgb(255, 180, 80),
            ErrorCategory::ServiceUnavailable { .. } => Color32::from_rgb(255, 160, 60),
            ErrorCategory::NonRetryable { is_permission_error, .. } => {
                if *is_permission_error {
                    Color32::from_rgb(255, 100, 100) // Red for permissions
                } else {
                    Color32::from_rgb(255, 120, 120)
                }
            }
        }
    }

    /// Reset the explorer state (called on Terminate action)
    /// Clears: resources, query scope, filters, tree state
    /// Preserves: cache (shared Moka cache is global), bookmarks (global)
    pub fn reset_state(&mut self) {
        if let Ok(mut state) = self.state.try_write() {
            // Clear resources and query results
            state.resources.clear();
            state.cached_queries.clear();

            // Reset query scope to empty
            state.query_scope = QueryScope::default();

            // Clear filters
            state.tag_filter_group = TagFilterGroup::new();
            state.property_filter_group =
                crate::app::resource_explorer::PropertyFilterGroup::new();
            state.search_filter.clear();

            // Reset loading state
            state.loading_tasks.clear();
            state.phase1_failed_queries.clear();
            state.phase1_tag_fetching = false;
            state.phase2_enrichment_in_progress = false;
            state.phase2_enrichment_completed = false;

            // Reset grouping to default
            state.primary_grouping = GroupingMode::ByAccount;

            tracing::info!("Explorer state reset (Terminate action)");
        }

        // Reset tree renderer state
        self.tree_renderer = TreeRenderer::new();
    }

    /// Set the AWS Identity Center reference to access real account data
    pub fn set_aws_identity_center(
        &mut self,
        aws_identity_center: Option<Arc<Mutex<AwsIdentityCenter>>>,
    ) {
        self.aws_identity_center = aws_identity_center.clone();

        // Initialize AWS client when Identity Center is available
        if let Some(identity_center_mutex) = aws_identity_center {
            if let Ok(identity_center_guard) = identity_center_mutex.try_lock() {
                // Extract the default role name from identity center
                let default_role = identity_center_guard.default_role_name.clone();
                drop(identity_center_guard); // Release the lock early

                self.tree_renderer
                    .set_default_role_name(Some(default_role.clone()));

                // Create credential coordinator with live reference to identity center
                let credential_coordinator = Arc::new(
                    crate::app::resource_explorer::credentials::CredentialCoordinator::new(
                        identity_center_mutex.clone(),
                        default_role,
                    ),
                );

                // Create AWS client with credential coordinator
                let aws_client = Arc::new(AWSResourceClient::new(credential_coordinator));
                self.aws_client = Some(aws_client.clone());

                // Set global AWS client for bridge tools
                set_global_aws_client(Some(aws_client));

                // Debounced logging - only log every 5 seconds (300 frames at 60fps)
                if self.frame_count % 300 == 0 {
                    tracing::debug!(
                        "🔧 AWS client created and set as global client for bridge tools"
                    );
                }
            }
        } else {
            self.tree_renderer.set_default_role_name(None);
            // Clear AWS client if identity center is removed
            self.aws_client = None;

            // Clear global AWS client for bridge tools
            set_global_aws_client(None);

            // Debounced logging - only log every 5 seconds (300 frames at 60fps)
            if self.frame_count % 300 == 0 {
                tracing::debug!("🔧 AWS client cleared from global bridge tools");
            }
        }
    }

    /// Get reference to the AWS client for use by other components
    pub fn get_aws_client(&self) -> Option<Arc<AWSResourceClient>> {
        self.aws_client.clone()
    }

    pub fn show(&mut self, ctx: &Context) -> WindowAction {
        if !self.is_open {
            return WindowAction::None;
        }

        // Reset minimize_requested flag at the start of each frame
        self.minimize_requested = false;

        // Poll V8 JavaScript action queue for showInExplorer() calls
        let v8_actions = super::drain_explorer_actions();
        for action in v8_actions {
            match action {
                super::ExplorerAction::OpenWithConfig(config) => {
                    self.apply_ephemeral_config(config);
                }
            }
        }

        if let Ok(mut updates) = self.console_role_menu_updates.lock() {
            for update in updates.drain(..) {
                self.tree_renderer.apply_console_role_menu_update(update);
            }
        }

        // Increment frame counter for debouncing
        self.frame_count += 1;

        // Request continuous repaints if we have active loading tasks to show spinner animation
        if let Ok(state) = self.state.try_read() {
            if state.is_loading() || state.phase2_enrichment_in_progress {
                // Request repaint every 100ms to keep spinner animated
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
        }

        // Check if Phase 2 enrichment completed and refresh resources from cache
        if let Ok(mut state) = self.state.try_write() {
            if state.phase2_enrichment_completed {
                // First, collect updates from cache (to avoid borrow conflicts)
                let updates: Vec<(
                    usize,
                    Option<serde_json::Value>,
                    Option<chrono::DateTime<chrono::Utc>>,
                )> = state
                    .resources
                    .iter()
                    .enumerate()
                    .filter_map(|(idx, resource)| {
                        if resource.detailed_properties.is_some() {
                            return None; // Already has details
                        }
                        let cache_key = format!(
                            "{}:{}:{}",
                            resource.account_id, resource.region, resource.resource_type
                        );
                        state
                            .cached_queries
                            .get(&cache_key)
                            .and_then(|cached_resources| {
                                cached_resources
                                    .iter()
                                    .find(|r| r.resource_id == resource.resource_id)
                                    .and_then(|cached| {
                                        cached.detailed_properties.as_ref().map(|props| {
                                            (idx, Some(props.clone()), cached.detailed_timestamp)
                                        })
                                    })
                            })
                    })
                    .collect();

                // Now apply updates
                let updated_count = updates.len();
                for (idx, props, timestamp) in updates {
                    if let Some(resource) = state.resources.get_mut(idx) {
                        resource.detailed_properties = props;
                        resource.detailed_timestamp = timestamp;
                    }
                }

                if updated_count > 0 {
                    tracing::info!(
                        "Phase 2 enrichment: Updated {} resources with detailed properties",
                        updated_count
                    );
                }
                // Reset flag so we don't do this every frame
                state.phase2_enrichment_completed = false;
            }
        }

        // Calculate window size based on screen dimensions
        let screen_rect = ctx.screen_rect();
        let top_bar_height = 60.0; // Approximate height of the top menu bar
        let window_width = 800.0;
        let window_height = screen_rect.height() - top_bar_height - 40.0; // Leave some margin

        // Track close button click
        let mut close_clicked = false;

        // Configure window with custom title bar for minimize/close buttons
        let _response = Window::new("AWS Explorer")
            .title_bar(false) // Custom title bar
            .default_size([window_width, window_height])
            .min_size([600.0, 400.0])
            .resizable(true)
            .movable(true)
            .collapsible(true)
            .constrain(true)
            .show(ctx, |ui| {
            // ================================================================
            // CUSTOM TITLE BAR - minimize and close buttons
            // ================================================================
            ui.horizontal(|ui| {
                ui.heading("AWS Explorer");

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // X (close) button - rightmost
                    if ui.button("X").on_hover_text("Close window").clicked() {
                        close_clicked = true;
                    }

                    // Minimize button - next to X
                    if ui.button("_").on_hover_text("Minimize - Hide window (keep state)").clicked() {
                        self.minimize_requested = true;
                    }
                });
            });
            ui.separator();
                // Check if this window has focus
                self.is_focused = ui
                    .ctx()
                    .memory(|mem| mem.focused().map(|id| id == ui.id()).unwrap_or(false));

                // Bottom panel for status bar and memory usage (prevents window from growing)
                egui::TopBottomPanel::bottom("explorer_status_bar")
                    .show_separator_line(true)
                    .show_inside(ui, |ui| {
                        // Status bar showing active operations
                        let status_line = global_status().get_status_line();
                        let is_active = status_line != "Ready";

                        ui.horizontal(|ui| {
                            // Use horizontal scroll to prevent long messages from expanding window width
                            // The scroll bar is invisible (no bar style) but allows content to overflow
                            egui::ScrollArea::horizontal()
                                .id_salt("status_bar_scroll")
                                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                                .show(ui, |ui| {

                            // Check for Phase 1 (resource listing) progress
                            let phase1_status = if let Ok(state) = self.state.try_read() {
                                if state.is_phase1_in_progress() {
                                    // Phase 1 in progress - clear old failed queries indicator and reset snapshot flag
                                    if !self.last_failed_queries.is_empty() {
                                        self.last_failed_queries.clear();
                                    }
                                    self.last_failed_queries_snapshotted = false; // Reset flag for next Phase 1 completion
                                    let (pending, total, failed, queries) = state.get_phase1_progress();
                                    Some((pending, total, queries, failed))
                                } else {
                                    // Phase 1 complete - snapshot failed queries with error categories ONCE
                                    // Only run this expensive operation once using the snapshotted flag
                                    if !state.phase1_failed_queries.is_empty() && !self.last_failed_queries_snapshotted {
                                        self.last_failed_queries.clear();

                                        for query_key in &state.phase1_failed_queries {
                                            // Query retry_tracker for error category
                                            if let Some(retry_state) = retry_tracker().get_query_state(query_key) {
                                                if let Some(error_category) = retry_state.last_error {
                                                    self.last_failed_queries.insert(
                                                        query_key.clone(),
                                                        error_category
                                                    );
                                                } else {
                                                    // Fallback: Query failed but no error category (shouldn't happen)
                                                    self.last_failed_queries.insert(
                                                        query_key.clone(),
                                                        ErrorCategory::NonRetryable {
                                                            code: "Unknown".to_string(),
                                                            message: "Query failed".to_string(),
                                                            is_permission_error: false,
                                                        }
                                                    );
                                                }
                                            } else {
                                                // Fallback: retry_tracker doesn't have this query
                                                self.last_failed_queries.insert(
                                                    query_key.clone(),
                                                    ErrorCategory::NonRetryable {
                                                        code: "Unknown".to_string(),
                                                        message: "Query failed".to_string(),
                                                        is_permission_error: false,
                                                    }
                                                );
                                            }
                                        }

                                        self.last_failed_queries_snapshotted = true; // Mark as snapshotted to prevent re-execution
                                    }
                                    None
                                }
                            } else {
                                None
                            };

                            // Check for Phase 1 tag fetching progress (during normalization)
                            let phase1_tag_status = if let Ok(state) = self.state.try_read() {
                                if state.phase1_tag_fetching {
                                    Some((
                                        state.phase1_tag_resource_type.clone().unwrap_or_else(|| "resources".to_string()),
                                        state.phase1_tag_progress_count,
                                        state.phase1_tag_progress_total,
                                    ))
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            // Check for Phase 1.5 (tag analysis) progress
                            let phase1_5_status = if let Ok(state) = self.state.try_read() {
                                if state.phase1_5_in_progress {
                                    Some((
                                        state.phase1_5_stage.clone().unwrap_or_else(|| "Analyzing".to_string()),
                                        state.phase1_5_progress_count,
                                        state.phase1_5_progress_total,
                                    ))
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            // Check for Phase 2 enrichment status
                            let phase2_status = if let Ok(state) = self.state.try_read() {
                                if state.phase2_enrichment_in_progress {
                                    let service = state.phase2_current_service.clone()
                                        .unwrap_or_else(|| "resources".to_string());
                                    Some((
                                        service,
                                        state.phase2_progress_count,
                                        state.phase2_progress_total,
                                    ))
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            // Get retry summary for status display
                            let retry_summary = retry_tracker().get_summary();

                            // Show Phase 1 progress if active (takes priority over Phase 2)
                            if let Some((pending, total, services, failed)) = phase1_status {
                                ui.spinner();
                                let completed = total - pending;
                                // Shorten resource type names for display
                                // "AWS::Lambda::Function" -> "Lambda"
                                let short_names: Vec<String> = services
                                    .iter()
                                    .map(|s| {
                                        // Extract just the service name from full query key
                                        // "account:region:AWS::Lambda::Function" -> "Lambda"
                                        let resource_type = s.split(':').last().unwrap_or(s);
                                        resource_type
                                            .strip_prefix("AWS::")
                                            .unwrap_or(resource_type)
                                            .split("::")
                                            .next()
                                            .unwrap_or(resource_type)
                                            .to_string()
                                    })
                                    .collect();
                                // Deduplicate service names (same service across regions)
                                let unique_names: std::collections::HashSet<_> = short_names.iter().cloned().collect();
                                let mut unique_list: Vec<_> = unique_names.into_iter().collect();
                                unique_list.sort();

                                // Show up to 3 pending services
                                let service_list = if unique_list.len() <= 3 {
                                    unique_list.join(", ")
                                } else {
                                    format!("{}, +{} more", unique_list[..3].join(", "), unique_list.len() - 3)
                                };
                                let message = format!(
                                    "Loading resources ({}/{})... {}",
                                    completed, total, service_list
                                );

                                ui.label(
                                    egui::RichText::new(&message)
                                        .color(Color32::from_rgb(255, 200, 100)) // Orange for Phase 1
                                        .small(),
                                );

                                // Show failed count as clickable indicator with tooltip
                                if failed > 0 {
                                    let failed_response = ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(format!("[{} unavailable]", failed))
                                                .color(Color32::from_rgb(255, 150, 50))
                                                .small()
                                        ).sense(egui::Sense::click())
                                    );

                                    if failed_response.clicked() {
                                        self.show_service_availability_dialog = true;
                                    }

                                    failed_response.on_hover_ui(|ui| {
                                        ui.label(egui::RichText::new("Service Availability").strong());
                                        ui.label("Some services are not available in all AWS regions.");
                                        ui.label("Click to see details.");
                                    });
                                }

                                // Add retry/throttle status if any
                                if retry_summary.active_retries > 0 || retry_summary.throttled_count > 0 {
                                    let retry_info = if retry_summary.throttled_count > 0 {
                                        format!("[{} throttled]", retry_summary.throttled_count)
                                    } else if retry_summary.timeout_count > 0 {
                                        format!("[{} timeout]", retry_summary.timeout_count)
                                    } else if retry_summary.network_error_count > 0 {
                                        format!("[{} network]", retry_summary.network_error_count)
                                    } else {
                                        format!("[{} retrying]", retry_summary.active_retries)
                                    };
                                    ui.label(
                                        egui::RichText::new(&retry_info)
                                            .color(Color32::from_rgb(255, 150, 50))
                                            .small(),
                                    );
                                }

                                // Show additional retry indicator if throttling is occurring
                                if retry_summary.throttled_count > 0 {
                                    ui.label(
                                        egui::RichText::new("(SDK retrying)")
                                            .color(Color32::from_rgb(255, 150, 50)) // Darker orange
                                            .small(),
                                    );
                                }

                                ui.ctx()
                                    .request_repaint_after(std::time::Duration::from_millis(100));
                            }
                            // Show Phase 1 tag fetching progress (during normalization)
                            else if let Some((resource_type, count, total)) = phase1_tag_status {
                                ui.spinner();
                                // Shorten resource type: "AWS::IAM::Role" -> "IAM Role"
                                let short_type = resource_type
                                    .strip_prefix("AWS::")
                                    .unwrap_or(&resource_type)
                                    .replace("::", " ");
                                let message = format!(
                                    "Fetching {} tags ({}/{})",
                                    short_type, count, total
                                );
                                ui.label(
                                    egui::RichText::new(&message)
                                        .color(Color32::from_rgb(255, 220, 150)) // Light orange for tag fetching
                                        .small(),
                                );
                                ui.ctx()
                                    .request_repaint_after(std::time::Duration::from_millis(100));
                            }
                            // Show Phase 1.5 progress if active (tag analysis between Phase 1 and Phase 2)
                            else if let Some((stage, count, total)) = phase1_5_status {
                                ui.spinner();
                                let message = format!(
                                    "{}... ({}/{})",
                                    stage, count, total
                                );
                                ui.label(
                                    egui::RichText::new(&message)
                                        .color(Color32::from_rgb(180, 255, 180)) // Light green for Phase 1.5
                                        .small(),
                                );
                                ui.ctx()
                                    .request_repaint_after(std::time::Duration::from_millis(100));
                            }
                            // Show Phase 2 progress if active
                            else if let Some((service, count, total)) = phase2_status {
                                // Animated spinner indicator
                                ui.spinner();

                                // Format service name for display (e.g., "AWS::S3::Bucket" -> "S3 Bucket")
                                let display_name = service
                                    .strip_prefix("AWS::")
                                    .unwrap_or(&service)
                                    .replace("::", " ");

                                let message = format!(
                                    "Enriching {} properties... ({}/{})",
                                    display_name, count, total
                                );
                                ui.label(
                                    egui::RichText::new(&message)
                                        .color(Color32::from_rgb(100, 180, 255))
                                        .small(),
                                );

                                // Request repaint to keep animation going
                                ui.ctx()
                                    .request_repaint_after(std::time::Duration::from_millis(50));
                            } else if is_active {
                                // Animated indicator for active operations
                                let time = ui.ctx().input(|i| i.time);
                                let pulse = ((time * 3.0).sin() * 0.3 + 0.7) as f32;
                                let color = Color32::from_rgba_unmultiplied(
                                    100,
                                    180,
                                    255,
                                    (255.0 * pulse) as u8,
                                );
                                ui.label(egui::RichText::new("*").color(color).strong());
                                ui.label(
                                    egui::RichText::new(&status_line)
                                        .color(Color32::from_rgb(100, 180, 255))
                                        .small(),
                                );

                                // Request repaint to keep animation going
                                ui.ctx()
                                    .request_repaint_after(std::time::Duration::from_millis(50));
                            } else {
                                // Show the standard status line (Ready, etc.)
                                ui.label(
                                    egui::RichText::new(&status_line)
                                        .color(Color32::GRAY)
                                        .small(),
                                );
                            }

                            // Always show availability indicator when there are failures (after any status)
                            // Use self.last_failed_queries which persists across frames after Phase 1 completes
                            // This shows alongside "Ready" or other status messages
                            let persistent_failed_count = self.last_failed_queries.len();
                            if persistent_failed_count > 0 {
                                let indicator_response = ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(format!("[{} queries failed]", persistent_failed_count))
                                            .color(Color32::from_rgb(255, 150, 50))
                                            .small()
                                    ).sense(egui::Sense::click())
                                );

                                if indicator_response.clicked() {
                                    self.show_service_availability_dialog = true;
                                }

                                indicator_response.on_hover_ui(|ui| {
                                    ui.label(egui::RichText::new("Failed Queries").strong());
                                    ui.separator();
                                    ui.label("Some queries failed due to errors or regional unavailability.");
                                    ui.label("This may be due to permissions, service availability, or network issues.");
                                    ui.add_space(4.0);
                                    ui.label(egui::RichText::new("Click to see details and error categories.").weak());
                                });
                            }

                            }); // End ScrollArea

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    // Get shared cache stats (Moka-based)
                                    let shared_cache = super::cache::shared_cache();
                                    shared_cache.run_pending_tasks(); // Ensure stats are up-to-date
                                    let cache_stats = shared_cache.memory_stats();

                                    if let Ok(state) = self.state.try_read() {
                                        let active_count = state.resources.len();

                                        // Get actual process memory usage
                                        if let Some(usage) = memory_stats::memory_stats() {
                                            let physical_mb =
                                                usage.physical_mem as f64 / (1024.0 * 1024.0);
                                            let cache_mb =
                                                cache_stats.total_size() as f64 / (1024.0 * 1024.0);
                                            let ratio = cache_stats.compression_ratio();

                                            // Show cache stats with compression info
                                            let cache_info = if cache_stats.resource_entry_count > 0 {
                                                format!(
                                                    "{:.1}MB cache ({:.1}x) | {} active, {} queries",
                                                    cache_mb,
                                                    ratio,
                                                    active_count,
                                                    cache_stats.resource_entry_count
                                                )
                                            } else {
                                                format!("{} active", active_count)
                                            };

                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{:.0}MB | {}",
                                                    physical_mb, cache_info
                                                ))
                                                .small()
                                                .color(Color32::GRAY),
                                            );
                                        } else {
                                            // Fallback if memory stats unavailable
                                            let cache_mb =
                                                cache_stats.total_size() as f64 / (1024.0 * 1024.0);
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{} active, {:.1}MB cached",
                                                    active_count, cache_mb
                                                ))
                                                .small()
                                                .color(Color32::GRAY),
                                            );
                                        }
                                    }
                                },
                            );
                        });
                    });

                // Left sidebar for grouping and filter controls
                egui::SidePanel::left("explorer_sidebar")
                    .default_width(180.0)
                    .min_width(150.0)
                    .resizable(true)
                    .show_inside(ui, |ui| {
                        if let Ok(mut state) = self.state.try_write() {
                            let actions = PaneRenderer::render_sidebar(ui, &mut state);
                            // Process sidebar actions (dialog triggers)
                            for action in actions {
                                match action {
                                    PaneAction::ShowTagFilterBuilder => {
                                        state.show_filter_builder = true;
                                    }
                                    PaneAction::ShowPropertyFilterBuilder => {
                                        state.show_property_filter_builder = true;
                                    }
                                    PaneAction::ShowTagHierarchyBuilder => {
                                        state.show_tag_hierarchy_builder = true;
                                    }
                                    PaneAction::ShowPropertyHierarchyBuilder => {
                                        state.show_property_hierarchy_builder = true;
                                    }
                                    // These actions are not expected from sidebar
                                    PaneAction::RemoveAccount(_)
                                    | PaneAction::RemoveRegion(_)
                                    | PaneAction::RemoveResourceType(_) => {}
                                }
                            }
                        }
                    });

                // Main content area
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    // Render unified toolbar with bookmarks menu and control buttons
                    let (clicked_bookmark_id, show_add, show_manage, clear_clicked) =
                        self.render_unified_toolbar(ui);

                    // Apply bookmark menu actions
                    if let Some(bookmark_id) = clicked_bookmark_id {
                        // Find the bookmark (read-only)
                        let bookmark_clone = self
                            .bookmark_manager
                            .read()
                            .unwrap()
                            .get_bookmarks()
                            .iter()
                            .find(|b| b.id == bookmark_id)
                            .cloned();

                        if let Some(bookmark) = bookmark_clone {
                            // Apply the bookmark to state (reconstructs full selections)
                            if let Ok(mut state) = self.state.try_write() {
                                self.apply_bookmark_to_state(&bookmark, &mut state, ctx);
                            }

                            // Update access tracking (separate borrow)
                            if let Some(bookmark_mut) = self
                                .bookmark_manager
                                .write()
                                .unwrap()
                                .get_bookmark_mut(&bookmark_id)
                            {
                                bookmark_mut.access_count += 1;
                                bookmark_mut.last_accessed = Some(chrono::Utc::now());
                                bookmark_mut.modified_at = chrono::Utc::now();
                            }

                            // Save updated bookmark with access tracking
                            if let Err(e) = self.bookmark_manager.write().unwrap().save() {
                                tracing::error!("Failed to save bookmark access tracking: {}", e);
                            }
                        }
                    }
                    if show_add {
                        self.show_bookmark_dialog = true;
                        tracing::info!("Add bookmark clicked");
                    }
                    if show_manage {
                        self.show_bookmark_manager = true;
                        tracing::info!("Manage bookmarks clicked");
                    }
                    if clear_clicked {
                        if let Ok(mut state) = self.state.try_write() {
                            state.clear_all_selections();
                        }
                    }

                    ui.separator();

                    if let Ok(mut state) = self.state.try_write() {
                        // Render active selection tags using PaneRenderer
                        let actions = PaneRenderer::render_active_tags(ui, &mut state);
                        // Process any removal actions
                        for action in actions {
                            match action {
                                PaneAction::RemoveAccount(id) => {
                                    tracing::info!("Removing account: {}", id);
                                    let was_phase2_running = state.phase2_enrichment_in_progress;
                                    state.remove_account(&id);
                                    self.handle_active_selection_reduction(&mut state, was_phase2_running);
                                }
                                PaneAction::RemoveRegion(code) => {
                                    tracing::info!("Removing region: {}", code);
                                    let was_phase2_running = state.phase2_enrichment_in_progress;
                                    state.remove_region(&code);
                                    self.handle_active_selection_reduction(&mut state, was_phase2_running);
                                }
                                PaneAction::RemoveResourceType(rt) => {
                                    tracing::info!("Removing resource type: {}", rt);
                                    let was_phase2_running = state.phase2_enrichment_in_progress;
                                    state.remove_resource_type(&rt);
                                    self.handle_active_selection_reduction(&mut state, was_phase2_running);
                                }
                                PaneAction::ShowTagFilterBuilder => {
                                    state.show_filter_builder = true;
                                }
                                PaneAction::ShowPropertyFilterBuilder => {
                                    state.show_property_filter_builder = true;
                                }
                                PaneAction::ShowTagHierarchyBuilder => {
                                    state.show_tag_hierarchy_builder = true;
                                }
                                PaneAction::ShowPropertyHierarchyBuilder => {
                                    state.show_property_hierarchy_builder = true;
                                }
                            }
                        }
                        ui.add_space(10.0);
                        PaneRenderer::render_search_bar(ui, &mut state);
                        ui.separator();
                        PaneRenderer::render_tree_view(ui, &state, &mut self.tree_renderer);
                    } else {
                        ui.label("Loading...");
                    }
                });
            });

        // Determine what action to take based on button clicks
        let action = if self.minimize_requested {
            // Minimize button was clicked - hide window but keep state
            self.is_open = false;
            WindowAction::Minimize
        } else if close_clicked {
            // X button was clicked - close and terminate (clear state)
            self.is_open = false;
            WindowAction::Terminate
        } else {
            // Window is still open, no action needed
            WindowAction::None
        };

        // Sync failed requests from window to tree renderer
        if let Ok(failed_set) = self.failed_detail_requests.try_read() {
            self.tree_renderer.failed_detail_requests = failed_set.clone();
        }

        // Process any pending detail requests from the tree renderer
        let pending_requests = if !self.tree_renderer.pending_detail_requests.is_empty() {
            Some(self.tree_renderer.pending_detail_requests.clone())
        } else {
            None
        };

        if let Some(requests) = pending_requests {
            self.tree_renderer.pending_detail_requests.clear();

            if let Ok(state) = self.state.try_read() {
                if let Some(ref aws_client) = self.aws_client {
                    PaneRenderer::process_pending_detail_requests(
                        &state,
                        ctx,
                        requests,
                        aws_client,
                        &self.state,
                        &self.failed_detail_requests,
                    );
                }
            }
        }

        // Process any pending tag badge clicks from the tree renderer
        let pending_tag_clicks = if !self.tree_renderer.pending_tag_clicks.is_empty() {
            Some(self.tree_renderer.pending_tag_clicks.clone())
        } else {
            None
        };

        if let Some(clicks) = pending_tag_clicks {
            self.tree_renderer.pending_tag_clicks.clear();

            if let Ok(mut state) = self.state.try_write() {
                PaneRenderer::process_tag_badge_clicks(&mut state, clicks);
            }
        }

        // Process any pending explorer actions from the tree renderer (e.g., open CloudWatch Logs)
        if !self.tree_renderer.pending_explorer_actions.is_empty() {
            if let Ok(mut actions) = self.pending_actions.lock() {
                actions.extend(self.tree_renderer.pending_explorer_actions.drain(..));
            }
        }

        // Handle dialogs outside the main window to avoid borrowing conflicts
        if let Ok(mut state) = self.state.try_write() {
            // Handle dialogs
            // Note: refresh dialog needs to be handled outside this lock scope due to &mut self requirement
            if state.show_account_dialog {
                // Get real accounts from AWS Identity Center instead of fake ones
                let available_accounts = self.get_available_accounts();
                if let Some(accounts) = self.fuzzy_dialog.show_account_dialog(
                    ctx,
                    &mut state.show_account_dialog,
                    &available_accounts,
                ) {
                    let count = accounts.len();
                    if count == 1 {
                        tracing::info!(
                            "🏢 Adding account: {} ({})",
                            accounts[0].display_name,
                            accounts[0].account_id
                        );
                    } else {
                        tracing::info!("🏢 Adding {} accounts", count);
                    }

                    for account in accounts {
                        state.add_account(account);
                    }

                    // Log current scope after addition
                    tracing::info!("📊 Current scope after adding accounts: {} accounts, {} regions, {} resource types",
                        state.query_scope.accounts.len(),
                        state.query_scope.regions.len(),
                        state.query_scope.resource_types.len());

                    if state.phase2_enrichment_in_progress {
                        state.cancel_phase2_enrichment();
                    }

                    // Trigger query if we have all required scope elements
                    self.trigger_query_if_ready(&state, ctx);
                }
            }
            if state.show_region_dialog {
                if let Some(regions) = self.fuzzy_dialog.show_region_dialog(
                    ctx,
                    &mut state.show_region_dialog,
                    &get_default_regions(),
                ) {
                    let count = regions.len();
                    if count == 1 {
                        tracing::info!(
                            "🌍 Adding region: {} ({})",
                            regions[0].display_name,
                            regions[0].region_code
                        );
                    } else {
                        tracing::info!("🌍 Adding {} regions", count);
                    }

                    for region in regions {
                        state.add_region(region);
                    }

                    // Log current scope after addition
                    tracing::info!("📊 Current scope after adding regions: {} accounts, {} regions, {} resource types",
                        state.query_scope.accounts.len(),
                        state.query_scope.regions.len(),
                        state.query_scope.resource_types.len());

                    if state.phase2_enrichment_in_progress {
                        state.cancel_phase2_enrichment();
                    }

                    // Trigger query if we have all required scope elements
                    self.trigger_query_if_ready(&state, ctx);
                }
            }
            if state.show_resource_type_dialog {
                if let Some(resource_types) = self.fuzzy_dialog.show_resource_type_dialog(
                    ctx,
                    &mut state.show_resource_type_dialog,
                    &get_default_resource_types(),
                ) {
                    let count = resource_types.len();
                    if count == 1 {
                        tracing::info!(
                            "📦 Adding resource type: {} ({})",
                            resource_types[0].display_name,
                            resource_types[0].resource_type
                        );
                    } else {
                        tracing::info!("📦 Adding {} resource types", count);
                    }

                    for resource_type in resource_types {
                        state.add_resource_type(resource_type);
                    }

                    // Log current scope after addition
                    tracing::info!("📊 Current scope after adding resource types: {} accounts, {} regions, {} resource types",
                        state.query_scope.accounts.len(),
                        state.query_scope.regions.len(),
                        state.query_scope.resource_types.len());

                    if state.phase2_enrichment_in_progress {
                        state.cancel_phase2_enrichment();
                    }

                    // Trigger query if we have all required scope elements
                    self.trigger_query_if_ready(&state, ctx);
                }
            }

            // Unified selection dialog (3-panel: Account | Region | Resource)
            if state.show_unified_selection_dialog {
                let available_accounts = self.get_available_accounts();

                // Get current selections to pre-populate the dialog
                let current_accounts = state.query_scope.accounts.clone();
                let current_regions = state.query_scope.regions.clone();
                let current_resources = state.query_scope.resource_types.clone();

                // Debug: log what we're passing to the dialog
                tracing::debug!(
                    "Opening unified selection dialog with: {} accounts, {} regions, {} resources",
                    current_accounts.len(),
                    current_regions.len(),
                    current_resources.len()
                );

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
                    // This allows users to both add and remove items

                    // Update accounts
                    state.query_scope.accounts.clear();
                    for account in accounts {
                        tracing::info!(
                            "Setting account: {} ({})",
                            account.display_name,
                            account.account_id
                        );
                        state.add_account(account);
                    }

                    // Update regions
                    state.query_scope.regions.clear();
                    for region in regions {
                        tracing::info!(
                            "Setting region: {} ({})",
                            region.display_name,
                            region.region_code
                        );
                        state.add_region(region);
                    }

                    // Update resource types
                    state.query_scope.resource_types.clear();
                    for resource in resources {
                        tracing::info!(
                            "Setting resource type: {} ({})",
                            resource.display_name,
                            resource.resource_type
                        );
                        state.add_resource_type(resource);
                    }

                    // Log current scope after update
                    tracing::info!(
                        "Current scope after unified selection: {} accounts, {} regions, {} resource types",
                        state.query_scope.accounts.len(),
                        state.query_scope.regions.len(),
                        state.query_scope.resource_types.len()
                    );

                    if state.phase2_enrichment_in_progress {
                        state.cancel_phase2_enrichment();
                    }

                    // Trigger query if we have all required scope elements
                    self.trigger_query_if_ready(&state, ctx);
                }
            }
        }

        // Sync refresh dialog state and handle dialog
        if let Ok(state) = self.state.try_read() {
            if state.show_refresh_dialog && !self.show_refresh_dialog {
                self.show_refresh_dialog = true;
            }
            if state.show_filter_builder && !self.show_filter_builder {
                self.show_filter_builder = true;
            }
            if state.show_property_filter_builder && !self.show_property_filter_builder {
                self.show_property_filter_builder = true;
            }
            if state.show_tag_hierarchy_builder && !self.show_hierarchy_builder {
                self.show_hierarchy_builder = true;
            }
            if state.show_property_hierarchy_builder && !self.show_property_hierarchy_builder {
                self.show_property_hierarchy_builder = true;
            }
        }

        if self.show_refresh_dialog {
            self.render_refresh_dialog_standalone(ctx);
        }

        if self.show_filter_builder {
            self.render_filter_builder_dialog(ctx);
        }

        if self.show_property_filter_builder {
            self.render_property_filter_builder_dialog(ctx);
        }

        if self.show_hierarchy_builder {
            self.render_hierarchy_builder_dialog(ctx);
        }

        if self.show_property_hierarchy_builder {
            self.render_property_hierarchy_builder_dialog(ctx);
        }

        // Bookmark dialogs
        if self.show_bookmark_dialog {
            self.render_bookmark_creation_dialog(ctx);
        }

        if self.show_bookmark_manager {
            self.render_bookmark_manager_dialog(ctx);
        }

        if self.show_bookmark_edit_dialog {
            self.render_bookmark_edit_dialog(ctx);
        }

        // Service availability dialog (shows failed queries due to regional availability)
        if self.show_service_availability_dialog {
            self.render_service_availability_dialog(ctx);
        }

        // Verification window (DEBUG builds only)
        #[cfg(debug_assertions)]
        {
            // Get credential coordinator from AWS client if available
            let credential_coordinator = self.aws_client.as_ref().map(|c| c.get_credential_coordinator());
            self.verification_window.show(ctx, &self.state, credential_coordinator.as_ref());
        }

        action
    }

    fn render_refresh_dialog_standalone(&mut self, ctx: &Context) {
        if !self.show_refresh_dialog {
            return;
        }

        let (combinations, display_to_cache) = if let Ok(state) = self.state.try_read() {
            self.generate_refresh_combinations(&state)
        } else {
            (Vec::new(), HashMap::new())
        };

        // Store the mapping for later use when refreshing
        self.refresh_display_to_cache = display_to_cache;

        let response = Window::new("Refresh AWS Resources")
            .default_size([500.0, 400.0])
            .resizable(true)
            .show(ctx, |ui| {
                ui.label("Select combinations to refresh:");
                ui.separator();

                if combinations.is_empty() {
                    ui.label("No query combinations available");
                } else {
                    // Selection buttons
                    ui.horizontal(|ui| {
                        if ui.button("Select All").clicked() {
                            for combo in &combinations {
                                self.refresh_selection.insert(combo.clone(), true);
                            }
                        }
                        if ui.button("Clear All").clicked() {
                            for combo in &combinations {
                                self.refresh_selection.insert(combo.clone(), false);
                            }
                        }
                    });

                    ui.separator();

                    // Show combinations with checkboxes in a scrollable area
                    egui::ScrollArea::vertical()
                        .max_height(250.0)
                        .show(ui, |ui| {
                            for combination in &combinations {
                                // Ensure the combination exists in our selection map
                                let is_selected =
                                    *self.refresh_selection.get(combination).unwrap_or(&false);

                                let mut selected = is_selected;
                                if ui.checkbox(&mut selected, combination).clicked() {
                                    self.refresh_selection.insert(combination.clone(), selected);
                                }
                            }
                        });

                    ui.separator();
                    ui.label(format!("Total combinations: {}", combinations.len()));

                    let selected_count = self.refresh_selection.values().filter(|&&v| v).count();
                    if selected_count > 0 {
                        ui.label(format!("Selected: {}", selected_count));
                    }
                }

                ui.separator();
                let buttons_response = ui.horizontal(|ui| {
                    let cancel_clicked = ui.button("Cancel").clicked();

                    let selected_count = self.refresh_selection.values().filter(|&&v| v).count();
                    let refresh_clicked = ui
                        .add_enabled(
                            selected_count > 0,
                            egui::Button::new(format!("Refresh {} Selected", selected_count)),
                        )
                        .clicked();

                    (cancel_clicked, refresh_clicked)
                });

                buttons_response.inner
            });

        if let Some(inner_response) = response {
            if let Some((cancel_clicked, refresh_clicked)) = inner_response.inner {
                if cancel_clicked {
                    self.show_refresh_dialog = false;
                    self.refresh_selection.clear();
                    self.refresh_display_to_cache.clear();
                    // Also clear the state flag
                    if let Ok(mut state) = self.state.try_write() {
                        state.show_refresh_dialog = false;
                    }
                } else if refresh_clicked {
                    // Trigger refresh for selected combinations
                    self.trigger_selective_refresh(ctx);
                    self.show_refresh_dialog = false;
                    self.refresh_selection.clear();
                    self.refresh_display_to_cache.clear();
                    // Also clear the state flag
                    if let Ok(mut state) = self.state.try_write() {
                        state.show_refresh_dialog = false;
                    }
                }
            }
        }
    }

    fn render_filter_builder_dialog(&mut self, ctx: &Context) {
        if !self.show_filter_builder {
            return;
        }

        // Initialize working group from state on first open
        if self.filter_builder_working_group.is_none() {
            if let Ok(state) = self.state.try_read() {
                self.filter_builder_working_group = Some(state.tag_filter_group.clone());
            } else {
                return;
            }
        }

        // Get tag discovery from state
        let tag_discovery = if let Ok(state) = self.state.try_read() {
            state.tag_discovery.clone()
        } else {
            return;
        };

        // Take the working group (we'll put it back after rendering)
        let working_group = self.filter_builder_working_group.take().unwrap();

        // Create the widget with the working group
        let mut widget = TagFilterBuilderWidget::new(working_group, tag_discovery);

        let response = Window::new("Advanced Tag Filter Builder")
            .open(&mut self.show_filter_builder)
            .default_size([700.0, 500.0])
            .resizable(true)
            .vscroll(true) // Let the Window itself handle scrolling
            .show(ctx, |ui| {
                // Render the widget directly (no ScrollArea to avoid clipping ComboBox popups)
                let updated_filter_group = widget.show(ui);

                ui.separator();

                let buttons_response = ui.horizontal(|ui| {
                    let cancel_clicked = ui.button("Cancel").clicked();
                    let apply_clicked = ui.button("Apply Filters").clicked();
                    (cancel_clicked, apply_clicked)
                });

                (updated_filter_group, buttons_response.inner)
            });

        if let Some(inner_response) = response {
            if let Some((updated_filter_group, (cancel_clicked, apply_clicked))) =
                inner_response.inner
            {
                // Check if X button was clicked (window open state changed to false)
                let x_clicked = !self.show_filter_builder;

                if cancel_clicked || x_clicked {
                    // Clear working group and close dialog (Cancel or X button)
                    self.filter_builder_working_group = None;
                    self.show_filter_builder = false;
                    // Clear the state flag
                    if let Ok(mut state) = self.state.try_write() {
                        state.show_filter_builder = false;
                    }
                } else if apply_clicked {
                    // Apply changes to state and close dialog
                    if let Ok(mut state) = self.state.try_write() {
                        // Log the filter expression for visibility
                        let filter_expr = TagFilterBuilderWidget::format_filter_expression(
                            &updated_filter_group,
                            0,
                        );
                        tracing::info!("Applying tag filter: {}", filter_expr);

                        state.tag_filter_group = updated_filter_group.clone();
                        state.show_filter_builder = false;
                    }

                    // Clear working group and close dialog
                    self.filter_builder_working_group = None;
                    self.show_filter_builder = false;
                } else {
                    // Neither button clicked - save working group for next frame
                    self.filter_builder_working_group = Some(updated_filter_group);
                }
            }
        } else {
            // Window was not shown - clear working group
            self.filter_builder_working_group = None;
            self.show_filter_builder = false;
            if let Ok(mut state) = self.state.try_write() {
                state.show_filter_builder = false;
            }
        }
    }

    fn render_property_filter_builder_dialog(&mut self, ctx: &Context) {
        if !self.show_property_filter_builder {
            return;
        }

        // Initialize working group from state on first open
        if self.property_filter_builder_working_group.is_none() {
            if let Ok(state) = self.state.try_read() {
                self.property_filter_builder_working_group =
                    Some(state.property_filter_group.clone());
            } else {
                return;
            }
        }

        // Get property catalog from state
        let property_catalog = if let Ok(state) = self.state.try_read() {
            state.property_catalog.clone()
        } else {
            return;
        };

        // Take the working group (we'll put it back after rendering)
        let working_group = self.property_filter_builder_working_group.take().unwrap();

        // Create the widget with the working group
        let mut widget = crate::app::resource_explorer::widgets::PropertyFilterBuilderWidget::new(
            working_group,
            property_catalog,
        );

        let response = Window::new("Property Filter Builder")
            .open(&mut self.show_property_filter_builder)
            .default_size([800.0, 600.0])
            .resizable(true)
            .vscroll(true)
            .show(ctx, |ui| {
                // Render the widget
                let updated_filter_group = widget.show(ui);

                ui.separator();

                let buttons_response = ui.horizontal(|ui| {
                    let cancel_clicked = ui.button("Cancel").clicked();
                    let apply_clicked = ui.button("Apply Filters").clicked();
                    (cancel_clicked, apply_clicked)
                });

                (updated_filter_group, buttons_response.inner)
            });

        if let Some(inner_response) = response {
            if let Some((updated_filter_group, (cancel_clicked, apply_clicked))) =
                inner_response.inner
            {
                // Check if X button was clicked
                let x_clicked = !self.show_property_filter_builder;

                if cancel_clicked || x_clicked {
                    // Clear working group and close dialog
                    self.property_filter_builder_working_group = None;
                    self.show_property_filter_builder = false;
                    // Clear the state flag
                    if let Ok(mut state) = self.state.try_write() {
                        state.show_property_filter_builder = false;
                    }
                } else if apply_clicked {
                    // Apply changes to state and close dialog
                    if let Ok(mut state) = self.state.try_write() {
                        tracing::info!(
                            "Applying property filter: {}",
                            updated_filter_group.description()
                        );

                        state.property_filter_group = updated_filter_group.clone();
                        state.show_property_filter_builder = false;
                    }

                    // Clear working group and close dialog
                    self.property_filter_builder_working_group = None;
                    self.show_property_filter_builder = false;
                } else {
                    // Neither button clicked - save working group for next frame
                    self.property_filter_builder_working_group = Some(updated_filter_group);
                }
            }
        } else {
            // Window was not shown - clear working group
            self.property_filter_builder_working_group = None;
            self.show_property_filter_builder = false;
            if let Ok(mut state) = self.state.try_write() {
                state.show_property_filter_builder = false;
            }
        }
    }

    fn render_hierarchy_builder_dialog(&mut self, ctx: &Context) {
        if !self.show_hierarchy_builder {
            return;
        }

        // Initialize widget once when dialog opens
        if self.hierarchy_builder_widget.is_none() {
            if let Ok(state) = self.state.try_read() {
                // Extract tag keys from current grouping mode if it's a hierarchy
                let initial_hierarchy = match &state.primary_grouping {
                    GroupingMode::ByTagHierarchy(keys) => keys.clone(),
                    _ => Vec::new(),
                };

                let tag_discovery = state.tag_discovery.clone();

                // Create widget instance - this will persist across frames
                self.hierarchy_builder_widget = Some(TagHierarchyBuilderWidget::new(
                    tag_discovery,
                    initial_hierarchy,
                ));

                tracing::info!("Tag hierarchy builder widget created");
            } else {
                return;
            }
        }

        // Get mutable reference to the persistent widget
        let widget = if let Some(widget) = &mut self.hierarchy_builder_widget {
            widget
        } else {
            return;
        };

        let response = Window::new("Configure Tag Hierarchy")
            .open(&mut self.show_hierarchy_builder)
            .default_size([900.0, 600.0])
            .resizable(true)
            .vscroll(false) // Widget handles its own scrolling
            .show(ctx, |ui| {
                // Render the persistent widget - it maintains state across frames
                widget.show(ui)
            });

        if let Some(inner_response) = response {
            if let Some((updated_hierarchy, apply_clicked, cancel_clicked)) = inner_response.inner {
                // Check if X button was clicked (window open state changed to false)
                let x_clicked = !self.show_hierarchy_builder;

                if cancel_clicked || x_clicked {
                    // Destroy widget and close dialog (Cancel or X button)
                    self.hierarchy_builder_widget = None;
                    self.show_hierarchy_builder = false;
                    tracing::info!("Tag hierarchy builder cancelled, widget destroyed");

                    // Clear the state flag
                    if let Ok(mut state) = self.state.try_write() {
                        state.show_tag_hierarchy_builder = false;
                    }
                } else if apply_clicked {
                    // Apply changes to state and close dialog
                    if let Ok(mut state) = self.state.try_write() {
                        // Log the hierarchy for visibility
                        let hierarchy_text = updated_hierarchy.join(" > ");
                        tracing::info!("Applying tag hierarchy: {}", hierarchy_text);

                        // Set the new grouping mode
                        state.primary_grouping =
                            GroupingMode::ByTagHierarchy(updated_hierarchy.clone());
                        state.show_tag_hierarchy_builder = false;
                    }

                    // Destroy widget and close dialog
                    self.hierarchy_builder_widget = None;
                    self.show_hierarchy_builder = false;
                    tracing::info!("Tag hierarchy applied, widget destroyed");
                }
                // If neither button clicked, widget persists with its current state
            }
        } else {
            // Window was not shown - destroy widget
            self.hierarchy_builder_widget = None;
            self.show_hierarchy_builder = false;
            tracing::info!("Tag hierarchy builder closed, widget destroyed");

            if let Ok(mut state) = self.state.try_write() {
                state.show_tag_hierarchy_builder = false;
            }
        }
    }

    fn render_property_hierarchy_builder_dialog(&mut self, ctx: &Context) {
        if !self.show_property_hierarchy_builder {
            return;
        }

        // Initialize widget once when dialog opens
        if self.property_hierarchy_builder_widget.is_none() {
            if let Ok(state) = self.state.try_read() {
                // Extract property paths from current grouping mode if it's a hierarchy
                let initial_hierarchy = match &state.primary_grouping {
                    GroupingMode::ByPropertyHierarchy(paths) => paths.clone(),
                    _ => Vec::new(),
                };

                let property_catalog = state.property_catalog.clone();

                // Create widget instance - this will persist across frames
                self.property_hierarchy_builder_widget = Some(PropertyHierarchyBuilderWidget::new(
                    property_catalog,
                    initial_hierarchy,
                ));

                tracing::info!("Property hierarchy builder widget created");
            } else {
                return;
            }
        }

        // Get mutable reference to the persistent widget
        let widget = if let Some(widget) = &mut self.property_hierarchy_builder_widget {
            widget
        } else {
            return;
        };

        let response = Window::new("Configure Property Hierarchy")
            .open(&mut self.show_property_hierarchy_builder)
            .default_size([900.0, 600.0])
            .resizable(true)
            .vscroll(false) // Widget handles its own scrolling
            .show(ctx, |ui| {
                // Render the persistent widget - it maintains state across frames
                widget.show(ui)
            });

        if let Some(inner_response) = response {
            if let Some((updated_hierarchy, apply_clicked, cancel_clicked)) = inner_response.inner {
                // Check if X button was clicked (window open state changed to false)
                let x_clicked = !self.show_property_hierarchy_builder;

                if cancel_clicked || x_clicked {
                    // Destroy widget and close dialog (Cancel or X button)
                    self.property_hierarchy_builder_widget = None;
                    self.show_property_hierarchy_builder = false;
                    tracing::info!("Property hierarchy builder cancelled, widget destroyed");

                    // Clear the state flag
                    if let Ok(mut state) = self.state.try_write() {
                        state.show_property_hierarchy_builder = false;
                    }
                } else if apply_clicked {
                    // Apply changes to state and close dialog
                    if let Ok(mut state) = self.state.try_write() {
                        // Log the hierarchy for visibility
                        let hierarchy_text = updated_hierarchy.join(" > ");
                        tracing::info!("Applying property hierarchy: {}", hierarchy_text);

                        // Set the new grouping mode
                        state.primary_grouping =
                            GroupingMode::ByPropertyHierarchy(updated_hierarchy.clone());
                        state.show_property_hierarchy_builder = false;
                    }

                    // Destroy widget and close dialog
                    self.property_hierarchy_builder_widget = None;
                    self.show_property_hierarchy_builder = false;
                    tracing::info!("Property hierarchy applied, widget destroyed");
                }
                // If neither button clicked, widget persists with its current state
            }
        } else {
            // Window was not shown - destroy widget
            self.property_hierarchy_builder_widget = None;
            self.show_property_hierarchy_builder = false;
            tracing::info!("Property hierarchy builder closed, widget destroyed");

            if let Ok(mut state) = self.state.try_write() {
                state.show_property_hierarchy_builder = false;
            }
        }
    }

    /// Render unified toolbar combining bookmarks menu and control buttons
    /// Returns: (clicked_bookmark_id, show_add_dialog, show_manage_dialog, clear_clicked)
    fn render_unified_toolbar(&mut self, ui: &mut Ui) -> (Option<String>, bool, bool, bool) {
        let mut clicked_bookmark_id: Option<String> = None;
        let mut show_add_dialog = false;
        let mut show_manage_dialog = false;
        let mut clear_clicked = false;

        ui.horizontal(|ui| {
            // Bookmarks menu (needs read-only state access)
            if let Ok(state) = self.state.try_read() {
                ui.menu_button("Bookmarks", |ui| {
                    // Render top-level bookmarks and folders
                    self.render_bookmark_menu_level(
                        ui,
                        None, // Top level (no parent folder)
                        &state,
                        &mut clicked_bookmark_id,
                    );

                    // Separator before management actions
                    ui.separator();

                    // Management actions at bottom of menu
                    if ui.button("Add Bookmark").clicked() {
                        show_add_dialog = true;
                        ui.close();
                    }

                    if ui.button("Manage Bookmarks").clicked() {
                        show_manage_dialog = true;
                        ui.close();
                    }
                });
            }

            // Separator between Bookmarks and control buttons
            ui.separator();

            // Toolbar buttons (need mutable state access for dialog flags)
            if let Ok(mut state) = self.state.try_write() {
                // Main "Select" button opens unified selection dialog
                if ui.button("Select").clicked() {
                    state.show_unified_selection_dialog = true;
                }

                // Dropdown menu for individual selection dialogs (power user shortcuts)
                ui.menu_button("v", |ui| {
                    if ui.button("Add Account").clicked() {
                        state.show_account_dialog = true;
                        ui.close();
                    }
                    if ui.button("Add Region").clicked() {
                        state.show_region_dialog = true;
                        ui.close();
                    }
                    if ui.button("Add Resource").clicked() {
                        state.show_resource_type_dialog = true;
                        ui.close();
                    }
                });

                ui.separator();

                if ui.button("Refresh").clicked() {
                    state.show_refresh_dialog = true;
                }

                if ui
                    .button("Reset")
                    .on_hover_text("Reset all selections to default state")
                    .clicked()
                {
                    clear_clicked = true;
                }

                // Verify with CLI button (DEBUG builds only)
                #[cfg(debug_assertions)]
                {
                    ui.separator();
                    if ui
                        .button("Verify with CLI")
                        .on_hover_text("Compare cached resources with AWS CLI output")
                        .clicked()
                    {
                        self.verification_window.open = true;
                    }
                }

                // Cache menu with memory info and clear option
                ui.separator();
                ui.menu_button("Cache", |ui| {
                    let shared_cache = super::cache::shared_cache();
                    shared_cache.run_pending_tasks();
                    let stats = shared_cache.memory_stats();

                    // Memory stats display
                    let compressed_mb = stats.total_size() as f64 / (1024.0 * 1024.0);
                    let uncompressed_mb = stats.total_uncompressed_size as f64 / (1024.0 * 1024.0);
                    let ratio = stats.compression_ratio();

                    ui.label(format!("Resource queries: {}", stats.resource_entry_count));
                    ui.label(format!("Detailed entries: {}", stats.detailed_entry_count));
                    ui.separator();
                    ui.label(format!("Compressed: {:.1} MB", compressed_mb));
                    ui.label(format!("Uncompressed: {:.1} MB", uncompressed_mb));
                    ui.label(format!("Compression: {:.1}x", ratio));
                    ui.separator();

                    if ui.button("Clear Cache").clicked() {
                        shared_cache.clear();
                        tracing::info!("User cleared resource cache");
                        ui.close();
                    }
                });

                // Show loading indicator if queries are active
                if state.is_loading() {
                    ui.separator();
                    ui.spinner();
                    ui.label(format!(
                        "Loading... ({} queries)",
                        state.loading_tasks.len()
                    ));
                }
            }

            // Minimize button moved to custom title bar
        });

        (
            clicked_bookmark_id,
            show_add_dialog,
            show_manage_dialog,
            clear_clicked,
        )
    }

    /// Recursively render a level of the bookmark menu hierarchy
    fn render_bookmark_menu_level(
        &self,
        ui: &mut Ui,
        parent_folder_id: Option<String>,
        state: &ResourceExplorerState,
        clicked_bookmark_id: &mut Option<String>,
    ) {
        // Get bookmarks at this level
        let bookmarks: Vec<_> = self
            .bookmark_manager
            .read()
            .unwrap()
            .get_bookmarks_in_folder(parent_folder_id.as_ref())
            .into_iter()
            .cloned()
            .collect();

        // Render bookmarks
        for bookmark in &bookmarks {
            let is_active = bookmark.matches_state(state);
            let button_text = if is_active {
                format!("[Active] {}", bookmark.name)
            } else {
                bookmark.name.clone()
            };

            let response = if is_active {
                ui.add(egui::Button::new(&button_text).fill(ui.visuals().selection.bg_fill))
            } else {
                ui.button(&button_text)
            };

            if response.clicked() {
                *clicked_bookmark_id = Some(bookmark.id.clone());
                ui.close();
            }

            // Show tooltip with bookmark details
            response.on_hover_ui(|ui| {
                ui.label(format!("Bookmark: {}", bookmark.name));
                if let Some(desc) = &bookmark.description {
                    ui.label(format!("Description: {}", desc));
                }
                ui.separator();
                ui.label(format!("Accounts: {}", bookmark.account_ids.len()));
                ui.label(format!("Regions: {}", bookmark.region_codes.len()));
                ui.label(format!(
                    "Resource Types: {}",
                    bookmark.resource_type_ids.len()
                ));
                ui.label(format!("Grouping: {:?}", bookmark.grouping));
                ui.separator();
                ui.label(format!("Used {} times", bookmark.access_count));
            });
        }

        // Get folders at this level
        let folders: Vec<_> = self
            .bookmark_manager
            .read()
            .unwrap()
            .get_subfolders(parent_folder_id.as_ref())
            .into_iter()
            .cloned()
            .collect();

        // Show separator between bookmarks and folders if both exist
        if !bookmarks.is_empty() && !folders.is_empty() {
            ui.separator();
        }

        // Render folders as nested submenus
        for folder in &folders {
            ui.menu_button(format!("Folder: {}", folder.name), |ui| {
                // Recursively render folder contents
                self.render_bookmark_menu_level(
                    ui,
                    Some(folder.id.clone()),
                    state,
                    clicked_bookmark_id,
                );
            });
        }

        // Show "empty" message if no bookmarks or folders at this level
        if bookmarks.is_empty() && folders.is_empty() {
            ui.label(egui::RichText::new("(no bookmarks)").italics().weak());
        }
    }

    /// Render bookmark creation dialog
    fn render_bookmark_creation_dialog(&mut self, ctx: &Context) {
        if !self.show_bookmark_dialog {
            return;
        }

        let mut should_create = false;

        let response = Window::new("Create Bookmark")
            .default_size([500.0, 200.0])
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label("Save current Explorer configuration as a bookmark");
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut self.bookmark_dialog_name);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Description:");
                        ui.text_edit_singleline(&mut self.bookmark_dialog_description);
                    });

                    ui.add_space(10.0);

                    // Folder selector
                    ui.horizontal(|ui| {
                        ui.label("Folder:");
                        let current_folder_name =
                            if let Some(folder_id) = &self.bookmark_dialog_folder_id {
                                self.bookmark_manager
                                    .read()
                                    .unwrap()
                                    .get_folder(folder_id)
                                    .map(|f| f.name.clone())
                                    .unwrap_or_else(|| "Top Folder".to_string())
                            } else {
                                "Top Folder".to_string()
                            };

                        egui::ComboBox::from_label("")
                            .selected_text(current_folder_name)
                            .show_ui(ui, |ui| {
                                if ui
                                    .selectable_label(
                                        self.bookmark_dialog_folder_id.is_none(),
                                        "Top Folder",
                                    )
                                    .clicked()
                                {
                                    self.bookmark_dialog_folder_id = None;
                                }

                                // Show all folders as options
                                for folder in self
                                    .bookmark_manager
                                    .read()
                                    .unwrap()
                                    .get_all_folders()
                                    .iter()
                                {
                                    let is_selected =
                                        self.bookmark_dialog_folder_id.as_ref() == Some(&folder.id);
                                    if ui.selectable_label(is_selected, &folder.name).clicked() {
                                        self.bookmark_dialog_folder_id = Some(folder.id.clone());
                                    }
                                }
                            });
                    });

                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button("Create").clicked() {
                            should_create = true;
                        }

                        if ui.button("Cancel").clicked() {
                            self.show_bookmark_dialog = false;
                            self.bookmark_dialog_name.clear();
                            self.bookmark_dialog_description.clear();
                            self.bookmark_dialog_folder_id = None;
                        }
                    });
                });
            });

        // Handle bookmark creation
        if should_create && !self.bookmark_dialog_name.is_empty() {
            if let Ok(state) = self.state.try_read() {
                let mut bookmark = Bookmark::new(self.bookmark_dialog_name.clone(), &state);
                if !self.bookmark_dialog_description.is_empty() {
                    bookmark.description = Some(self.bookmark_dialog_description.clone());
                }
                // Set the folder_id
                bookmark.folder_id = self.bookmark_dialog_folder_id.clone();

                self.bookmark_manager
                    .write()
                    .unwrap()
                    .add_bookmark(bookmark);

                let folder_name = if let Some(folder_id) = &self.bookmark_dialog_folder_id {
                    self.bookmark_manager
                        .read()
                        .unwrap()
                        .get_folder(folder_id)
                        .map(|f| format!(" in folder '{}'", f.name))
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                tracing::info!(
                    "Created bookmark: {}{}",
                    self.bookmark_dialog_name,
                    folder_name
                );

                // Save bookmarks to disk
                if let Err(e) = self.bookmark_manager.write().unwrap().save() {
                    tracing::error!("Failed to save bookmarks: {}", e);
                }
            }

            self.show_bookmark_dialog = false;
            self.bookmark_dialog_name.clear();
            self.bookmark_dialog_description.clear();
            self.bookmark_dialog_folder_id = None;
        }

        // Handle window close via X button
        if response.is_none() {
            self.show_bookmark_dialog = false;
            self.bookmark_dialog_name.clear();
            self.bookmark_dialog_description.clear();
            self.bookmark_dialog_folder_id = None;
        }
    }

    /// Render the edit bookmark dialog
    fn render_bookmark_edit_dialog(&mut self, ctx: &Context) {
        if !self.show_bookmark_edit_dialog {
            return;
        }

        let mut should_save = false;

        let response = Window::new("Edit Bookmark")
            .default_size([500.0, 200.0])
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label("Edit bookmark name and description");
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut self.bookmark_edit_name);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Description:");
                        ui.text_edit_singleline(&mut self.bookmark_edit_description);
                    });

                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            should_save = true;
                        }

                        if ui.button("Cancel").clicked() {
                            self.show_bookmark_edit_dialog = false;
                            self.editing_bookmark_id = None;
                            self.bookmark_edit_name.clear();
                            self.bookmark_edit_description.clear();
                        }
                    });
                });
            });

        // Handle bookmark update
        if should_save && !self.bookmark_edit_name.is_empty() {
            if let Some(bookmark_id) = &self.editing_bookmark_id {
                if let Some(bookmark) = self
                    .bookmark_manager
                    .write()
                    .unwrap()
                    .get_bookmark_mut(bookmark_id)
                {
                    bookmark.name = self.bookmark_edit_name.clone();
                    bookmark.description = if self.bookmark_edit_description.is_empty() {
                        None
                    } else {
                        Some(self.bookmark_edit_description.clone())
                    };
                    bookmark.modified_at = chrono::Utc::now();

                    tracing::info!("Updated bookmark: {}", bookmark.name);
                }

                // Save bookmarks to disk
                if let Err(e) = self.bookmark_manager.write().unwrap().save() {
                    tracing::error!("Failed to save bookmarks: {}", e);
                }
            }

            self.show_bookmark_edit_dialog = false;
            self.editing_bookmark_id = None;
            self.bookmark_edit_name.clear();
            self.bookmark_edit_description.clear();
        }

        // Handle window close via X button
        if response.is_none() {
            self.show_bookmark_edit_dialog = false;
            self.editing_bookmark_id = None;
            self.bookmark_edit_name.clear();
            self.bookmark_edit_description.clear();
        }
    }

    /// Render the failed queries dialog showing which queries failed and their error categories
    fn render_service_availability_dialog(&mut self, ctx: &Context) {
        let response = Window::new("Failed Queries")
            .default_size([450.0, 350.0])
            .collapsible(false)
            .resizable(true)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // Header with explanation
                    ui.label(
                        egui::RichText::new("The following queries failed to complete")
                            .strong()
                    );
                    ui.add_space(4.0);
                    ui.label(
                        "Queries may fail due to various reasons including service unavailability \
                         in a region, permissions issues, network errors, or rate limiting."
                    );
                    ui.add_space(8.0);

                    ui.separator();

                    // Group failed queries by service type for clarity
                    let mut by_service: std::collections::HashMap<String, Vec<(String, String, ErrorCategory)>> =
                        std::collections::HashMap::new();

                    for (query_key, error_category) in &self.last_failed_queries {
                        let parts: Vec<&str> = query_key.split(':').collect();
                        if parts.len() >= 3 {
                            let account = parts[0].to_string();
                            let region = parts[1].to_string();
                            let resource_type = parts[2..].join(":");

                            // Shorten resource type: "AWS::BedrockAgentCore::AgentRuntime" -> "BedrockAgentCore AgentRuntime"
                            let short_name = resource_type
                                .strip_prefix("AWS::")
                                .unwrap_or(&resource_type)
                                .replace("::", " ");

                            by_service
                                .entry(short_name)
                                .or_default()
                                .push((account, region, error_category.clone()));
                        }
                    }

                    // Sort services alphabetically
                    let mut services: Vec<_> = by_service.keys().cloned().collect();
                    services.sort();

                    // Display grouped by service
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            for service in &services {
                                if let Some(locations) = by_service.get(service) {
                                    egui::CollapsingHeader::new(
                                        egui::RichText::new(format!("{} ({} regions)", service, locations.len()))
                                            .color(Color32::from_rgb(255, 180, 100))
                                    )
                                    .default_open(services.len() == 1) // Auto-expand if only one service
                                    .show(ui, |ui| {
                                        for (account, region, error_category) in locations {
                                            let error_label = Self::error_category_label(error_category);
                                            let error_color = Self::error_category_color(error_category);

                                            ui.horizontal(|ui| {
                                                ui.label(format!("  {} (Account: {})", region, account));
                                                ui.label(
                                                    egui::RichText::new(format!("[{}]", error_label))
                                                        .color(error_color)
                                                        .small()
                                                );
                                            });
                                        }
                                    });
                                }
                            }
                        });

                    ui.add_space(8.0);
                    ui.separator();

                    // Common causes and troubleshooting tips
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Common Causes:").strong());
                        });
                        ui.label("  dispatch failure - Service endpoint not available in region");
                        ui.label("  failed to construct request - Invalid service configuration");
                        ui.label("  Permission Denied - IAM policy missing required permissions");
                        ui.label("  Rate Limited - Too many API requests");
                        ui.label("  Timeout - Request exceeded time limit");
                        ui.label("  Service Unavailable - Temporary AWS service disruption");
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Tip:").strong());
                            ui.label("Check AWS Regional Services list and IAM policies.");
                        });
                    });

                    ui.add_space(8.0);

                    // Close button
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("Close").clicked() {
                                self.show_service_availability_dialog = false;
                            }
                            if ui.button("Dismiss (hide indicator)").clicked() {
                                self.show_service_availability_dialog = false;
                                self.last_failed_queries.clear(); // Clear to hide the indicator
                                self.last_failed_queries_snapshotted = false; // Reset snapshot flag
                                // Also clear state's failed queries to prevent re-snapshot
                                if let Ok(mut state) = self.state.try_write() {
                                    state.phase1_failed_queries.clear();
                                }
                            }
                        });
                    });
                });
            });

        // Handle window close via X button
        if response.is_none() {
            self.show_service_availability_dialog = false;
        }
    }

    /// Apply a bookmark to the current state, reconstructing full selections from IDs
    fn apply_bookmark_to_state(
        &self,
        bookmark: &Bookmark,
        state: &mut ResourceExplorerState,
        ctx: &Context,
    ) {
        tracing::info!("Applying bookmark '{}' to Explorer state", bookmark.name);

        // Reset Phase 2 state from any previous bookmark
        state.cancel_phase2_enrichment();

        // Clear existing query scope
        state.query_scope.accounts.clear();
        state.query_scope.regions.clear();
        state.query_scope.resource_types.clear();

        // Rebuild AccountSelection objects from stored account IDs
        let available_accounts = self.get_available_accounts();
        for account_id in &bookmark.account_ids {
            if let Some(aws_account) = available_accounts
                .iter()
                .find(|a| &a.account_id == account_id)
            {
                let account_sel =
                    AccountSelection::new(account_id.clone(), aws_account.account_name.clone());
                state.add_account(account_sel);
                tracing::debug!(
                    "  OK: Restored account: {} ({})",
                    aws_account.account_name,
                    account_id
                );
            } else {
                tracing::warn!(
                    "  WARN: Account {} not found in available accounts, skipping",
                    account_id
                );
            }
        }

        // Rebuild RegionSelection objects from stored region codes
        for region_code in &bookmark.region_codes {
            let display_name = Self::format_region_display_name(region_code);
            let region_sel = RegionSelection::new(region_code.clone(), display_name.clone());
            state.add_region(region_sel);
            tracing::debug!("  OK: Restored region: {} ({})", display_name, region_code);
        }

        // Rebuild ResourceTypeSelection objects from stored resource type IDs
        let available_types = get_default_resource_types();
        for resource_type_id in &bookmark.resource_type_ids {
            if let Some(res_type) = available_types
                .iter()
                .find(|rt| &rt.resource_type == resource_type_id)
            {
                state.add_resource_type(res_type.clone());
                tracing::debug!(
                    "  OK: Restored resource type: {} ({})",
                    res_type.display_name,
                    resource_type_id
                );
            } else {
                tracing::warn!(
                    "  WARN: Resource type {} not found in available types, skipping",
                    resource_type_id
                );
            }
        }

        // Apply other state components
        state.primary_grouping = bookmark.grouping.clone();
        state.tag_filter_group = bookmark.tag_filters.clone();
        state.search_filter = bookmark.search_filter.clone();

        tracing::info!(
            "  → Restored: {} accounts, {} regions, {} resource types, grouping: {:?}",
            state.query_scope.accounts.len(),
            state.query_scope.regions.len(),
            state.query_scope.resource_types.len(),
            state.primary_grouping
        );

        // Trigger query with restored scope if we have all required elements
        self.trigger_query_if_ready(state, ctx);
    }

    /// Format region code into human-readable display name
    fn format_region_display_name(region_code: &str) -> String {
        // Special case for global
        if region_code == "Global" || region_code == "global" {
            return "Global".to_string();
        }

        // Parse AWS region code format: us-east-1 → US East (N. Virginia)
        let parts: Vec<&str> = region_code.split('-').collect();
        if parts.len() >= 2 {
            let geo = match parts[0] {
                "us" => "US",
                "eu" => "EU",
                "ap" => "Asia Pacific",
                "ca" => "Canada",
                "sa" => "South America",
                "af" => "Africa",
                "me" => "Middle East",
                _ => parts[0],
            };

            let direction = match parts[1] {
                "east" => "East",
                "west" => "West",
                "north" => "North",
                "south" => "South",
                "central" => "Central",
                "northeast" => "Northeast",
                "southeast" => "Southeast",
                _ => parts[1],
            };

            let number = if parts.len() > 2 { parts[2] } else { "" };

            if number.is_empty() {
                format!("{} {}", geo, direction)
            } else {
                format!("{} {} {}", geo, direction, number)
            }
        } else {
            // Fallback to original if parsing fails
            region_code.to_string()
        }
    }

    /// Render bookmark management dialog
    fn render_bookmark_manager_dialog(&mut self, ctx: &Context) {
        if !self.show_bookmark_manager {
            return;
        }

        let mut bookmark_to_delete: Option<String> = None;
        let mut bookmark_to_edit: Option<String> = None;
        let mut folder_to_delete: Option<String> = None;
        let mut folder_to_rename: Option<String> = None;
        let mut move_bookmark_to_folder: Option<(String, Option<String>)> = None; // (bookmark_id, folder_id)
        let mut is_drag_drop_move = false; // Track if this is a drag-drop operation (always move, not copy)

        let response = Window::new("Manage Bookmarks")
            .default_size([700.0, 500.0])
            .resizable(true)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // Stats
                    ui.horizontal(|ui| {
                        ui.label(format!("Total bookmarks: {}", self.bookmark_manager.read().unwrap().get_bookmarks().len()));
                        ui.add_space(10.0);
                        ui.label(format!("Total folders: {}", self.bookmark_manager.read().unwrap().get_all_folders().len()));
                    });

                    // Toolbar
                    ui.horizontal(|ui| {
                        if ui.button("New Folder").clicked() {
                            self.show_folder_dialog = true;
                            self.folder_dialog_name = String::new();
                            self.folder_dialog_parent_id = None;
                            self.editing_folder_id = None;
                        }
                    });

                    ui.separator();

                    egui::ScrollArea::vertical()
                        .max_height(350.0)
                        .show(ui, |ui| {
                            // Add "Top Folder" drop zone
                            let top_folder_response = ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("🗁 Top Folder").strong());
                            });

                            // Check if something is being dragged over Top Folder
                            if let Some(_dragged_data) = top_folder_response.response.dnd_hover_payload::<DragData>() {
                                // Always allow dropping into Top Folder (any item can be dropped here)

                                // Visual feedback: highlight Top Folder
                                let painter = ui.painter();
                                painter.rect_stroke(
                                    top_folder_response.response.rect,
                                    3.0,
                                    egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 200, 255)),
                                    egui::epaint::StrokeKind::Outside,
                                );
                            }

                            // Handle drop on Top Folder
                            if let Some(dropped_data) = top_folder_response.response.dnd_release_payload::<DragData>() {
                                match dropped_data.as_ref() {
                                    DragData::Bookmark { id, source_folder } => {
                                        // Don't drop bookmark if it's already in Top Folder
                                        if source_folder.is_some() {
                                            move_bookmark_to_folder = Some((id.clone(), None));
                                            is_drag_drop_move = true; // Drag-drop should always move
                                        }
                                    }
                                    DragData::Folder { id: dragged_folder_id, parent_id: current_parent } => {
                                        // Don't drop folder if it's already in Top Folder
                                        if current_parent.is_some() {
                                            // Move folder to Top Folder (parent_id = None)
                                            if let Err(e) = self.bookmark_manager.write().unwrap().move_folder_to_parent(dragged_folder_id, None) {
                                                tracing::error!("Failed to move folder to Top Folder: {}", e);
                                            } else if let Err(e) = self.bookmark_manager.write().unwrap().save() {
                                                tracing::error!("Failed to save after folder move to Top Folder: {}", e);
                                            }
                                        }
                                    }
                                }
                            }

                            ui.add_space(5.0);

                            // Render Top Folder level folders and bookmarks
                            self.render_folder_tree_level(
                                ui,
                                None,
                                &mut bookmark_to_delete,
                                &mut bookmark_to_edit,
                                &mut folder_to_delete,
                                &mut folder_to_rename,
                                &mut move_bookmark_to_folder,
                                &mut is_drag_drop_move,
                            );
                        });

                    ui.add_space(10.0);

                    if ui.button("Close").clicked() {
                        self.show_bookmark_manager = false;
                    }
                });
            });

        // Handle folder creation/edit dialog
        if self.show_folder_dialog {
            self.render_folder_dialog(ctx);
        }

        // Handle folder renaming
        if let Some(folder_id) = folder_to_rename {
            if let Some(folder) = self.bookmark_manager.read().unwrap().get_folder(&folder_id) {
                self.editing_folder_id = Some(folder.id.clone());
                self.folder_dialog_name = folder.name.clone();
                self.folder_dialog_parent_id = folder.parent_id.clone();
                self.show_folder_dialog = true;
            }
        }

        // Handle folder deletion
        if let Some(folder_id) = folder_to_delete {
            match self.bookmark_manager.write() {
                Ok(mut manager) => {
                    match manager.remove_folder(&folder_id) {
                        Ok(Some(removed)) => {
                            tracing::info!("Deleted folder: {}", removed.name);
                            self.expanded_folders.remove(&folder_id);

                            // Save while holding the lock
                            if let Err(e) = manager.save() {
                                tracing::error!("Failed to save folders: {}", e);
                            }
                        }
                        Err(e) => {
                            tracing::error!("Cannot delete folder: {}", e);
                        }
                        _ => {}
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to acquire write lock for folder deletion: {}", e);
                }
            }
        }

        // Handle bookmark moving/copying to folder OR folder moving
        if let Some((item_id, target_folder_id)) = move_bookmark_to_folder {
            // Check if this is a bookmark or a folder
            let is_bookmark = self
                .bookmark_manager
                .read()
                .unwrap()
                .get_bookmarks()
                .iter()
                .any(|b| b.id == item_id);

            if is_bookmark {
                // Handle bookmark move/copy
                if is_drag_drop_move || self.bookmark_clipboard_is_cut {
                    // Drag-drop or Cut: Move the bookmark to the new folder
                    self.bookmark_manager
                        .write()
                        .unwrap()
                        .move_bookmark_to_folder(&item_id, target_folder_id);
                } else {
                    // Copy: Duplicate the bookmark and place the copy in the new folder
                    let original = self
                        .bookmark_manager
                        .read()
                        .unwrap()
                        .get_bookmarks()
                        .iter()
                        .find(|b| b.id == item_id)
                        .cloned();

                    if let Some(original) = original {
                        let mut copied = original.clone();
                        copied.id = uuid::Uuid::new_v4().to_string();
                        copied.folder_id = target_folder_id;
                        copied.created_at = chrono::Utc::now();
                        self.bookmark_manager.write().unwrap().add_bookmark(copied);
                    }
                }

                // Clear clipboard
                self.bookmark_clipboard = None;
                self.bookmark_clipboard_is_cut = false;
            } else {
                // Handle folder move (drag-drop only, no clipboard for folders)
                if let Err(e) = self
                    .bookmark_manager
                    .write()
                    .unwrap()
                    .move_folder_to_parent(&item_id, target_folder_id)
                {
                    tracing::error!("Failed to move folder: {}", e);
                    // Show error to user (could add a toast/notification here)
                }
            }

            // Save to disk
            if let Err(e) = self.bookmark_manager.write().unwrap().save() {
                tracing::error!("Failed to save operation: {}", e);
            }
        }

        // Handle bookmark editing
        if let Some(bookmark_id) = bookmark_to_edit {
            let bookmark = self
                .bookmark_manager
                .read()
                .unwrap()
                .get_bookmarks()
                .iter()
                .find(|b| b.id == bookmark_id)
                .cloned();

            if let Some(bookmark) = bookmark {
                // Populate edit dialog fields
                self.editing_bookmark_id = Some(bookmark.id.clone());
                self.bookmark_edit_name = bookmark.name.clone();
                self.bookmark_edit_description = bookmark.description.clone().unwrap_or_default();
                self.show_bookmark_edit_dialog = true;
                tracing::info!("Opening edit dialog for bookmark: {}", bookmark.name);
            }
        }

        // Handle bookmark deletion
        if let Some(bookmark_id) = bookmark_to_delete {
            tracing::debug!("Attempting to delete bookmark: {}", bookmark_id);

            // Use match to handle potential lock poisoning
            match self.bookmark_manager.write() {
                Ok(mut manager) => {
                    if let Some(removed) = manager.remove_bookmark(&bookmark_id) {
                        tracing::info!("Deleted bookmark: {}", removed.name);

                        // Save inline while we still hold the lock
                        tracing::debug!("Saving bookmarks after deletion...");
                        if let Err(e) = manager.save() {
                            tracing::error!("Failed to save bookmarks: {}", e);
                        } else {
                            tracing::debug!("Bookmarks saved successfully");
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to acquire write lock for bookmark deletion: {}", e);
                }
            }
            tracing::debug!("Bookmark deletion handling complete");
        }

        // Handle window close via X button
        if response.is_none() {
            self.show_bookmark_manager = false;
        }
    }

    /// Recursively render a folder tree level
    #[allow(clippy::too_many_arguments)]
    fn render_folder_tree_level(
        &mut self,
        ui: &mut Ui,
        parent_id: Option<String>,
        bookmark_to_delete: &mut Option<String>,
        bookmark_to_edit: &mut Option<String>,
        folder_to_delete: &mut Option<String>,
        folder_to_rename: &mut Option<String>,
        move_bookmark_to_folder: &mut Option<(String, Option<String>)>,
        is_drag_drop_move: &mut bool,
    ) {
        // Get folders at this level
        let folders = self
            .bookmark_manager
            .read()
            .unwrap()
            .get_subfolders(parent_id.as_ref())
            .iter()
            .map(|f| (*f).clone())
            .collect::<Vec<_>>();

        // Render folders
        for folder in &folders {
            let folder_id = folder.id.clone();
            let is_expanded = self.expanded_folders.contains(&folder_id);

            // Horizontal layout: [drag handle] [folder header]
            let row_response = ui.horizontal(|ui| {
                // Drag handle - only this small area is draggable
                let folder_drag_id = ui.id().with("folder_drag").with(&folder_id);
                let drag_payload = DragData::Folder {
                    id: folder_id.clone(),
                    parent_id: parent_id.clone(),
                };

                let _handle_response = ui.dnd_drag_source(folder_drag_id, drag_payload, |ui| {
                    ui.label(":: "); // Drag handle icon (same as bookmarks)
                });

                // Folder header - this stays interactive (collapse arrow works)
                let header_response =
                    egui::CollapsingHeader::new(format!("\u{1F5C1} {}", folder.name))
                        .id_salt(&folder_id)
                        .default_open(is_expanded)
                        .show(ui, |ui| {
                            // Recursively render subfolders and bookmarks
                            self.render_folder_tree_level(
                                ui,
                                Some(folder_id.clone()),
                                bookmark_to_delete,
                                bookmark_to_edit,
                                folder_to_delete,
                                folder_to_rename,
                                move_bookmark_to_folder,
                                is_drag_drop_move,
                            );
                        });

                // Track expansion state
                if header_response.body_returned.is_some() && !is_expanded {
                    self.expanded_folders.insert(folder_id.clone());
                } else if header_response.body_returned.is_none() && is_expanded {
                    self.expanded_folders.remove(&folder_id);
                }

                // Add context menu on right-click on the header
                let header_resp = header_response.header_response.clone();
                header_resp.context_menu(|ui| {
                    if let Some(clipboard_id) = &self.bookmark_clipboard {
                        if ui.button("Paste Bookmark Here").clicked() {
                            *move_bookmark_to_folder =
                                Some((clipboard_id.clone(), Some(folder_id.clone())));
                            ui.close();
                        }
                    } else {
                        ui.label(egui::RichText::new("(no bookmark copied)").weak().italics());
                    }

                    ui.separator();

                    if ui.button("Rename Folder").clicked() {
                        *folder_to_rename = Some(folder_id.clone());
                        ui.close();
                    }

                    if ui.button("Delete Folder").clicked() {
                        *folder_to_delete = Some(folder_id.clone());
                        ui.close();
                    }
                });

                header_response.header_response
            });

            // Check if something is being dragged over this folder row
            if let Some(dragged_data) = row_response.response.dnd_hover_payload::<DragData>() {
                let can_drop = match dragged_data.as_ref() {
                    DragData::Bookmark { source_folder, .. } => {
                        // Don't allow dropping bookmark on its own folder
                        source_folder.as_ref() != Some(&folder_id)
                    }
                    DragData::Folder {
                        id: dragged_folder_id,
                        ..
                    } => {
                        // Don't allow dropping folder on itself and prevent circular references
                        dragged_folder_id != &folder_id
                            && !self
                                .bookmark_manager
                                .read()
                                .unwrap()
                                .is_descendant(&folder_id, dragged_folder_id)
                    }
                };

                if can_drop {
                    // Visual feedback: highlight folder
                    let painter = ui.painter();
                    painter.rect_stroke(
                        row_response.response.rect,
                        3.0,
                        egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 200, 255)),
                        egui::epaint::StrokeKind::Outside,
                    );
                }
            }

            // Handle drop
            if let Some(dropped_data) = row_response.response.dnd_release_payload::<DragData>() {
                match dropped_data.as_ref() {
                    DragData::Bookmark { id, source_folder } => {
                        // Don't drop bookmark on its own folder
                        if source_folder.as_ref() != Some(&folder_id) {
                            *move_bookmark_to_folder = Some((id.clone(), Some(folder_id.clone())));
                            *is_drag_drop_move = true; // Drag-drop should always move
                        }
                    }
                    DragData::Folder {
                        id: dragged_folder_id,
                        ..
                    } => {
                        // Don't drop folder on itself and prevent circular references
                        if dragged_folder_id != &folder_id
                            && !self
                                .bookmark_manager
                                .read()
                                .unwrap()
                                .is_descendant(&folder_id, dragged_folder_id)
                        {
                            // Move folder to be child of this folder
                            if let Err(e) = self
                                .bookmark_manager
                                .write()
                                .unwrap()
                                .move_folder_to_parent(dragged_folder_id, Some(folder_id.clone()))
                            {
                                tracing::error!("Failed to move folder: {}", e);
                            } else if let Err(e) = self.bookmark_manager.write().unwrap().save() {
                                tracing::error!("Failed to save after folder move: {}", e);
                            }
                        }
                    }
                }
            }
        }

        // Get bookmarks at this level
        let mut bookmarks = self
            .bookmark_manager
            .read()
            .unwrap()
            .get_bookmarks_in_folder(parent_id.as_ref())
            .iter()
            .map(|b| (*b).clone())
            .collect::<Vec<_>>();

        // Render bookmarks with drag-and-drop support
        if !bookmarks.is_empty() {
            let dnd_id = format!(
                "bookmark_dnd_{}",
                parent_id.as_deref().unwrap_or("top_folder")
            );
            let dnd_response =
                dnd(ui, &dnd_id).show_vec(&mut bookmarks, |ui, bookmark, handle, _state| {
                    let bookmark_id = bookmark.id.clone();

                    // Render the bookmark content
                    let scope_response = ui.scope(|ui| {
                        ui.horizontal(|ui| {
                            // Drag handle - make ONLY the handle draggable for cross-folder moves
                            let bookmark_drag_id =
                                ui.id().with("bookmark_native_drag").with(&bookmark_id);
                            let drag_payload = DragData::Bookmark {
                                id: bookmark_id.clone(),
                                source_folder: parent_id.clone(),
                            };

                            // Wrap only the handle in native drag-drop
                            let _handle_response =
                                ui.dnd_drag_source(bookmark_drag_id, drag_payload, |ui| {
                                    handle.ui(ui, |ui| {
                                        ui.label(":: ");
                                    });
                                });

                            // Bold title - make it more prominent
                            let default_size = ui
                                .style()
                                .text_styles
                                .get(&egui::TextStyle::Body)
                                .map(|f| f.size)
                                .unwrap_or(14.0);
                            ui.label(
                                egui::RichText::new(&bookmark.name)
                                    .strong()
                                    .size(default_size * 1.1),
                            );

                            // Italic description
                            if let Some(desc) = &bookmark.description {
                                ui.label(egui::RichText::new(format!("- {}", desc)).italics());
                            }
                        });

                        // Smaller font for summary (20% smaller)
                        let default_size = ui
                            .style()
                            .text_styles
                            .get(&egui::TextStyle::Body)
                            .map(|f| f.size)
                            .unwrap_or(14.0);
                        let smaller_size = default_size * 0.8;

                        ui.label(
                            egui::RichText::new(format!(
                                "  {} accounts, {} regions, {} resource types | Used {} times",
                                bookmark.account_ids.len(),
                                bookmark.region_codes.len(),
                                bookmark.resource_type_ids.len(),
                                bookmark.access_count
                            ))
                            .size(smaller_size),
                        );

                        ui.separator();
                    });

                    // Create an interactive response for the bookmark area to enable context menu
                    let rect = scope_response.response.rect;
                    let interact_id = ui.id().with(&bookmark_id);
                    let interact_response = ui.interact(rect, interact_id, egui::Sense::click());

                    // Add context menu on right-click
                    interact_response.context_menu(|ui| {
                        if ui.button("Copy").clicked() {
                            self.bookmark_clipboard = Some(bookmark_id.clone());
                            self.bookmark_clipboard_is_cut = false;
                            ui.close();
                        }

                        if ui.button("Cut").clicked() {
                            self.bookmark_clipboard = Some(bookmark_id.clone());
                            self.bookmark_clipboard_is_cut = true;
                            ui.close();
                        }

                        ui.separator();

                        if ui.button("Edit").clicked() {
                            *bookmark_to_edit = Some(bookmark_id.clone());
                            ui.close();
                        }

                        if ui.button("Delete").clicked() {
                            *bookmark_to_delete = Some(bookmark_id.clone());
                            ui.close();
                        }
                    });
                });

            // Handle drag-and-drop reordering within this folder
            if let Some(update) = dnd_response.final_update() {
                // Get all bookmarks in this folder (fresh from manager)
                let folder_bookmarks: Vec<_> = self
                    .bookmark_manager
                    .read()
                    .unwrap()
                    .get_bookmarks_in_folder(parent_id.as_ref())
                    .iter()
                    .map(|b| b.id.clone())
                    .collect();

                // Find the actual bookmark IDs being moved
                if update.from < folder_bookmarks.len() && update.to < folder_bookmarks.len() {
                    let from_id = &folder_bookmarks[update.from];
                    let to_id = &folder_bookmarks[update.to];

                    // Find indices in the global bookmark list
                    let all_bookmarks = self
                        .bookmark_manager
                        .read()
                        .unwrap()
                        .get_bookmarks()
                        .to_vec();
                    if let (Some(from_global), Some(to_global)) = (
                        all_bookmarks.iter().position(|b| &b.id == from_id),
                        all_bookmarks.iter().position(|b| &b.id == to_id),
                    ) {
                        self.bookmark_manager
                            .write()
                            .unwrap()
                            .reorder(from_global, to_global);

                        // Save to disk
                        if let Err(e) = self.bookmark_manager.write().unwrap().save() {
                            tracing::error!("Failed to save bookmark reorder: {}", e);
                        }
                    }
                }
            }
        }

        // Show "empty" message if no folders or bookmarks
        if folders.is_empty() && bookmarks.is_empty() {
            ui.label(egui::RichText::new("  (empty)").italics().weak());
        }
    }

    /// Render folder creation/edit dialog
    fn render_folder_dialog(&mut self, ctx: &Context) {
        if !self.show_folder_dialog {
            return;
        }

        let mut should_create = false;
        let is_editing = self.editing_folder_id.is_some();

        let title = if is_editing {
            "Edit Folder"
        } else {
            "New Folder"
        };

        let response = Window::new(title)
            .default_size([400.0, 200.0])
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label("Folder name:");
                    ui.text_edit_singleline(&mut self.folder_dialog_name);

                    ui.add_space(10.0);

                    ui.label("Parent folder:");
                    let current_parent_name = if let Some(parent_id) = &self.folder_dialog_parent_id
                    {
                        self.bookmark_manager
                            .read()
                            .unwrap()
                            .get_folder(parent_id)
                            .map(|f| f.name.clone())
                            .unwrap_or_else(|| "Top Folder".to_string())
                    } else {
                        "Top Folder".to_string()
                    };

                    egui::ComboBox::from_label("")
                        .selected_text(current_parent_name)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(
                                    self.folder_dialog_parent_id.is_none(),
                                    "Top Folder",
                                )
                                .clicked()
                            {
                                self.folder_dialog_parent_id = None;
                            }

                            // Show all folders as potential parents (except the one being edited)
                            for folder in self
                                .bookmark_manager
                                .read()
                                .unwrap()
                                .get_all_folders()
                                .iter()
                            {
                                if self.editing_folder_id.as_ref() != Some(&folder.id) {
                                    let is_selected =
                                        self.folder_dialog_parent_id.as_ref() == Some(&folder.id);
                                    if ui.selectable_label(is_selected, &folder.name).clicked() {
                                        self.folder_dialog_parent_id = Some(folder.id.clone());
                                    }
                                }
                            }
                        });

                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui
                            .button(if is_editing { "Update" } else { "Create" })
                            .clicked()
                            && !self.folder_dialog_name.is_empty()
                        {
                            should_create = true;
                        }

                        if ui.button("Cancel").clicked() {
                            self.show_folder_dialog = false;
                            self.editing_folder_id = None;
                        }
                    });
                });
            });

        // Create or update folder
        if should_create {
            if let Some(editing_id) = &self.editing_folder_id {
                // Update existing folder
                if let Some(folder) = self
                    .bookmark_manager
                    .write()
                    .unwrap()
                    .get_folder_mut(editing_id)
                {
                    folder.name = self.folder_dialog_name.clone();
                    folder.parent_id = self.folder_dialog_parent_id.clone();
                    folder.modified_at = chrono::Utc::now();

                    tracing::info!("Updated folder: {}", folder.name);
                }

                // Save to disk
                if let Err(e) = self.bookmark_manager.write().unwrap().save() {
                    tracing::error!("Failed to save folder update: {}", e);
                }
            } else {
                // Create new folder
                let folder = BookmarkFolder::new(
                    self.folder_dialog_name.clone(),
                    self.folder_dialog_parent_id.clone(),
                );

                tracing::info!("Created folder: {}", folder.name);
                self.bookmark_manager.write().unwrap().add_folder(folder);

                // Save to disk
                if let Err(e) = self.bookmark_manager.write().unwrap().save() {
                    tracing::error!("Failed to save folder: {}", e);
                }
            }

            self.show_folder_dialog = false;
            self.editing_folder_id = None;
        }

        // Handle window close via X button
        if response.is_none() {
            self.show_folder_dialog = false;
            self.editing_folder_id = None;
        }
    }

    fn generate_refresh_combinations(
        &self,
        state: &ResourceExplorerState,
    ) -> (Vec<String>, HashMap<String, String>) {
        let mut display_combinations = Vec::new();
        let mut display_to_cache_key = HashMap::new();

        for account in &state.query_scope.accounts {
            for region in &state.query_scope.regions {
                for resource_type in &state.query_scope.resource_types {
                    // Display name (friendly, shown to user)
                    let display_name = format!(
                        "{}/{}/{}",
                        account.display_name, region.display_name, resource_type.display_name
                    );

                    // Cache key (actual IDs, used internally)
                    let cache_key = format!(
                        "{}:{}:{}",
                        account.account_id, region.region_code, resource_type.resource_type
                    );

                    display_combinations.push(display_name.clone());
                    display_to_cache_key.insert(display_name, cache_key);
                }
            }
        }

        (display_combinations, display_to_cache_key)
    }

    pub fn is_open(&self) -> bool {
        self.is_open
    }

    pub fn set_open(&mut self, open: bool) {
        self.is_open = open;
    }

    pub fn focus(&mut self, ctx: &Context) {
        self.is_focused = true;
        ctx.request_repaint();
    }

    pub fn is_focused(&self) -> bool {
        self.is_focused
    }

    pub fn handle_key_event(
        &mut self,
        _ctx: &Context,
        _key: egui::Key,
        _modifiers: egui::Modifiers,
    ) -> bool {
        // TODO: Implement keyboard shortcuts
        false
    }

    /// Apply ephemeral configuration from V8 showInExplorer() call
    /// Opens the Explorer window and applies the specified configuration
    fn apply_ephemeral_config(
        &mut self,
        config: crate::app::agent_framework::v8_bindings::bindings::resources::ShowInExplorerArgs,
    ) {
        use crate::app::resource_explorer::colors::{assign_account_color, assign_region_color};
        use crate::app::resource_explorer::state::*;

        // Open the window if not already open
        self.is_open = true;

        // Get write lock on state to apply configuration
        let state_arc = self.state.clone();
        let mut attempts = 0;
        while attempts < 10 {
            if let Ok(mut state) = state_arc.try_write() {
                // Apply account selection if provided
                if let Some(account_ids) = config.accounts {
                    // Match account IDs to AccountSelection from identity center
                    if let Some(ref identity) = self.aws_identity_center {
                        if let Ok(identity_guard) = identity.lock() {
                            let accounts: Vec<AccountSelection> = account_ids
                                .iter()
                                .filter_map(|id| {
                                    identity_guard
                                        .accounts
                                        .iter()
                                        .find(|acc| &acc.account_id == id)
                                        .map(|acc| AccountSelection {
                                            account_id: acc.account_id.clone(),
                                            display_name: acc.account_name.clone(),
                                            color: assign_account_color(&acc.account_id),
                                        })
                                })
                                .collect();
                            if !accounts.is_empty() {
                                state.query_scope.accounts = accounts;
                            }
                        }
                    }
                }

                // Apply region selection if provided
                if let Some(region_codes) = config.regions {
                    let regions: Vec<RegionSelection> = region_codes
                        .iter()
                        .map(|code| RegionSelection {
                            region_code: code.clone(),
                            display_name: code.clone(), // V8 provides codes only
                            color: assign_region_color(code),
                        })
                        .collect();
                    if !regions.is_empty() {
                        state.query_scope.regions = regions;
                    }
                }

                // Apply resource type selection if provided
                if let Some(resource_types) = config.resource_types {
                    let types: Vec<ResourceTypeSelection> = resource_types
                        .iter()
                        .map(|rt| {
                            // Extract service name from CloudFormation type (AWS::EC2::Instance -> EC2)
                            let service_name = rt
                                .split("::")
                                .nth(1)
                                .unwrap_or("Unknown")
                                .to_string();

                            ResourceTypeSelection {
                                resource_type: rt.clone(),
                                display_name: rt.clone(), // V8 provides CloudFormation types
                                service_name,
                            }
                        })
                        .collect();
                    if !types.is_empty() {
                        state.query_scope.resource_types = types;
                    }
                }

                // Apply grouping if provided
                if let Some(grouping) = config.grouping {
                    // Convert from V8 GroupingMode to state GroupingMode via serde
                    if let Ok(json) = serde_json::to_value(&grouping) {
                        if let Ok(converted) = serde_json::from_value(json) {
                            state.primary_grouping = converted;
                        }
                    }
                }

                // Apply tag filters if provided
                if let Some(tag_filters) = config.tag_filters {
                    // Convert from V8 TagFilterGroup to state TagFilterGroup via serde
                    if let Ok(json) = serde_json::to_value(&tag_filters) {
                        if let Ok(converted) = serde_json::from_value(json) {
                            state.tag_filter_group = converted;
                        }
                    }
                }

                // Apply search filter if provided
                if let Some(search) = config.search_filter {
                    state.search_filter = search;
                }

                tracing::info!(
                    "Applied ephemeral config from V8: {} accounts, {} regions, {} types",
                    state.query_scope.accounts.len(),
                    state.query_scope.regions.len(),
                    state.query_scope.resource_types.len()
                );

                return;
            }
            attempts += 1;
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        warn!("Failed to acquire write lock for apply_ephemeral_config after 10 attempts");
    }

    /// Trigger AWS resource query if all required scope elements are present
    /// Uses parallel querying for real-time results
    fn trigger_query_if_ready(&self, state: &ResourceExplorerState, ctx: &Context) {
        // Only log when we actually have scope to avoid flooding logs
        if !state.query_scope.is_empty() && !state.is_loading() {
            tracing::info!(
                "✅ Triggering parallel query for {} combinations",
                state.query_scope.accounts.len()
                    * state.query_scope.regions.len()
                    * state.query_scope.resource_types.len()
            );
            // Clone state for async operation
            let state_arc = self.state.clone();
            let scope = state.query_scope.clone();
            // Use shared Moka cache instead of wrapping state.cached_queries
            let cache = super::cache::shared_cache();

            // Clone AWS client for thread
            let aws_client = match &self.aws_client {
                Some(client) => client.clone(),
                None => {
                    warn!("AWS client not available - AWS Identity Center may not be configured");
                    return;
                }
            };

            // Mark as loading and request UI repaint
            let cache_key = if let Ok(mut loading_state) = self.state.try_write() {
                loading_state.start_loading_task("parallel_query")
            } else {
                // Fallback if we can't get the lock
                format!(
                    "parallel_query_fallback_{}",
                    chrono::Utc::now().timestamp_millis()
                )
            };

            // Force UI repaint to show spinner immediately and schedule continuous updates
            ctx.request_repaint_after(std::time::Duration::from_millis(50));

            Self::spawn_parallel_query(state_arc, scope, cache, aws_client, cache_key);
        }
    }

    fn spawn_parallel_query(
        state_arc: Arc<RwLock<ResourceExplorerState>>,
        scope: QueryScope,
        cache: Arc<super::cache::SharedResourceCache>,
        aws_client: Arc<AWSResourceClient>,
        cache_key: String,
    ) {
        // Build list of query keys to track for Phase 1 progress
        // Each key is "account_id:region:resource_type" to match actual query granularity
        // This prevents race conditions where one region completing marks the entire service as done
        let global_registry = super::GlobalServiceRegistry::new();
        let mut queries_to_track: Vec<String> = Vec::new();

        for account in &scope.accounts {
            for resource_type in &scope.resource_types {
                if global_registry.is_global(&resource_type.resource_type) {
                    // Global services: one query per account with region "Global"
                    let query_key = ResourceExplorerState::make_query_key(
                        &account.account_id,
                        "Global",
                        &resource_type.resource_type,
                    );
                    if !queries_to_track.contains(&query_key) {
                        queries_to_track.push(query_key);
                    }
                } else {
                    // Regional services: one query per account × region
                    for region in &scope.regions {
                        let query_key = ResourceExplorerState::make_query_key(
                            &account.account_id,
                            &region.region_code,
                            &resource_type.resource_type,
                        );
                        queries_to_track.push(query_key);
                    }
                }
            }
        }

        // Initialize Phase 1 tracking - this is critical for progress tracking
        // Try a few times on the main thread, but the thread will retry more aggressively
        for attempt in 0..5 {
            if let Ok(mut state) = state_arc.try_write() {
                state.start_phase1_tracking(queries_to_track.clone());
                if attempt > 0 {
                    tracing::debug!(
                        "Phase 1 tracking initialized on main thread after {} retries",
                        attempt
                    );
                }
                break;
            }
            // Brief yield
            std::thread::yield_now();
        }
        // If main thread couldn't initialize, the spawned thread will handle it

        // Clone queries for use in spawned thread (for retry initialization if needed)
        let queries_for_thread = queries_to_track;

        std::thread::spawn(move || {
            let runtime = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!("Failed to create Tokio runtime: {}", e);
                    if let Ok(mut state) = state_arc.try_write() {
                        state.loading_tasks.remove(&cache_key);
                        state.reset_phase1_state();
                    }
                    return;
                }
            };

            // Initialize Phase 1 tracking inside thread - this MUST succeed before queries start
            // The thread can afford to wait since it's not the UI thread
            let mut tracking_init_success = false;
            for attempt in 0..100 {
                if let Ok(mut state) = state_arc.try_write() {
                    // Check if tracking was already initialized by main thread
                    if state.phase1_total_queries == 0 && !queries_for_thread.is_empty() {
                        tracing::info!(
                            "📋 Phase 1: Initializing tracking inside thread for {} queries (attempt {})",
                            queries_for_thread.len(),
                            attempt
                        );
                        state.start_phase1_tracking(queries_for_thread.clone());
                    }
                    tracking_init_success = true;
                    break;
                }
                // Wait longer between retries - UI frame is ~16ms, so wait 5ms each
                std::thread::sleep(std::time::Duration::from_millis(5));
            }

            if !tracking_init_success {
                tracing::error!(
                    "!!! CRITICAL: Failed to initialize Phase 1 tracking after 100 retries (500ms)"
                );
                super::query_timing::log_query_event(
                    "!!! PHASE1 TRACKING: try_write() failed after 100 retries in thread - tracking not initialized"
                );
            }

            let result: Result<Vec<super::state::ResourceEntry>, anyhow::Error> = runtime
                .block_on(async {
                    let (result_sender, mut result_receiver) =
                        tokio::sync::mpsc::channel::<super::aws_client::QueryResult>(1000);
                    let (progress_sender, mut progress_receiver) =
                        tokio::sync::mpsc::channel::<super::aws_client::QueryProgress>(100);

                    let aws_client_clone = aws_client.clone();
                    let scope_clone = scope.clone();
                    let cache_clone = cache.clone();

                    let query_future = aws_client_clone.query_aws_resources_parallel(
                        &scope_clone,
                        result_sender,
                        Some(progress_sender),
                        cache_clone,
                    );

                    let all_resources = Arc::new(tokio::sync::Mutex::new(Vec::new()));
                    let all_resources_clone = all_resources.clone();
                    let state_arc_clone = state_arc.clone();

                    let result_processing = async {
                        while let Some(result) = result_receiver.recv().await {
                            // Build full query key for Phase 1 tracking
                            let query_key = ResourceExplorerState::make_query_key(
                                &result.account_id,
                                &result.region,
                                &result.resource_type,
                            );
                            match result.resources {
                                Ok(resources) => {
                                    {
                                        let mut all_res = all_resources_clone.lock().await;
                                        all_res.extend(resources);

                                        // Retry loop for lock contention - critical for status bar updates
                                        let max_retries = 10;
                                        let mut success = false;
                                        for attempt in 0..max_retries {
                                            if let Ok(mut state) = state_arc_clone.try_write() {
                                                state.resources = all_res.clone();
                                                state.mark_phase1_query_completed(&query_key);
                                                success = true;
                                                if attempt > 0 {
                                                    tracing::debug!(
                                                        "Lock acquired after {} retries for {}",
                                                        attempt, query_key
                                                    );
                                                }
                                                break;
                                            }
                                            // Small yield to let UI thread release the lock
                                            tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
                                        }
                                        if !success {
                                            tracing::error!(
                                                "!!! LOCK CONTENTION: Failed to mark {} as completed after {} retries!",
                                                query_key, max_retries
                                            );
                                            super::query_timing::log_query_event(&format!(
                                                "!!! LOCK CONTENTION: try_write() failed after {} retries for mark_phase1_query_completed({})",
                                                max_retries, query_key
                                            ));
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "Query failed for {}: {}",
                                        query_key,
                                        e
                                    );
                                    // Retry loop for failed queries too
                                    let max_retries = 10;
                                    let mut success = false;
                                    for attempt in 0..max_retries {
                                        if let Ok(mut state) = state_arc_clone.try_write() {
                                            state.mark_phase1_query_failed(&query_key);
                                            success = true;
                                            if attempt > 0 {
                                                tracing::debug!(
                                                    "Lock acquired after {} retries for failed {}",
                                                    attempt, query_key
                                                );
                                            }
                                            break;
                                        }
                                        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
                                    }
                                    if !success {
                                        tracing::error!(
                                            "!!! LOCK CONTENTION: Failed to mark {} as failed after {} retries!",
                                            query_key, max_retries
                                        );
                                        super::query_timing::log_query_event(&format!(
                                            "!!! LOCK CONTENTION: try_write() failed after {} retries for mark_phase1_query_failed({})",
                                            max_retries, query_key
                                        ));
                                    }
                                }
                            }
                        }
                        let final_count = all_resources_clone.lock().await.len();
                        tracing::debug!(
                            "Result receiver channel closed, collected {} total resources",
                            final_count
                        );
                    };

                    let state_arc_for_progress = state_arc.clone();
                    let progress_processing = async {
                        while let Some(progress) = progress_receiver.recv().await {
                            tracing::debug!(
                                "Progress: {} - {} - {} (status: {:?})",
                                progress.account,
                                progress.region,
                                progress.message,
                                progress.status
                            );

                            // Handle FetchingTags progress - update state for UI display
                            if matches!(progress.status, super::aws_client::QueryStatus::FetchingTags) {
                                tracing::info!(
                                    "FetchingTags progress: {} ({}/{})",
                                    progress.resource_type,
                                    progress.items_processed.unwrap_or(0),
                                    progress.estimated_total.unwrap_or(0)
                                );
                                if let Ok(mut state) = state_arc_for_progress.try_write() {
                                    state.phase1_tag_fetching = true;
                                    state.phase1_tag_resource_type = Some(progress.resource_type.clone());
                                    if let (Some(count), Some(total)) = (progress.items_processed, progress.estimated_total) {
                                        state.phase1_tag_progress_count = count;
                                        state.phase1_tag_progress_total = total;
                                    }
                                }
                            } else if matches!(progress.status,
                                super::aws_client::QueryStatus::Completed |
                                super::aws_client::QueryStatus::Failed) {
                                // Clear tag fetching state when any resource type completes
                                // With parallel queries, clearing unconditionally is safe because
                                // the next FetchingTags message will set it again if still fetching
                                if let Ok(mut state) = state_arc_for_progress.try_write() {
                                    tracing::debug!(
                                        "Clearing tag fetch state on {:?} for {}",
                                        progress.status,
                                        progress.resource_type
                                    );
                                    state.phase1_tag_fetching = false;
                                    state.phase1_tag_resource_type = None;
                                }
                            }
                        }
                        // Clear tag fetching state when channel closes (final cleanup)
                        if let Ok(mut state) = state_arc_for_progress.try_write() {
                            state.phase1_tag_fetching = false;
                            state.phase1_tag_resource_type = None;
                        }
                        tracing::debug!("Progress receiver channel closed");
                    };

                    tokio::join!(
                        async {
                            match query_future.await {
                                Ok(()) => {
                                    tracing::debug!("Query execution completed successfully")
                                }
                                Err(e) => {
                                    tracing::error!("Parallel query execution failed: {}", e)
                                }
                            }
                        },
                        result_processing,
                        progress_processing
                    );

                    let final_resources = all_resources.lock().await.clone();

                    Ok(final_resources)
                });

            match result {
                Ok(_query_resources) => {
                    for attempt in 0..3 {
                        if let Ok(mut state) = state_arc.try_write() {
                            // Sync from shared cache to state.cached_queries for compatibility
                            // (will be removed when state.cached_queries is fully deprecated)
                            let final_cache = cache.to_hashmap();
                            state.cached_queries = final_cache;

                            // Filter resources to match current scope before setting state.resources
                            // This ensures Phase 2 only enriches visible resources (e.g., S3 buckets
                            // in selected regions only, not all buckets from the global query)
                            let cached_queries = state.cached_queries.clone();
                            Self::refresh_resources_from_cache_filtered(&mut state, &cached_queries);

                            state.finish_loading_task(&cache_key);

                            // Reset Phase 1 tracking since query is complete
                            state.reset_phase1_state();

                            state.update_tag_popularity();

                            tracing::debug!(
                                "Parallel query completed: {} total resources after filtering (loading tasks remaining: {})",
                                state.resources.len(),
                                state.loading_task_count()
                            );

                            Self::maybe_start_phase2_enrichment_for_state(
                                &mut state,
                                state_arc.clone(),
                                aws_client.clone(),
                            );

                            break;
                        } else if attempt == 2 {
                            tracing::warn!(
                                "Failed to update state after query completion after 3 attempts"
                            );
                        } else {
                            std::thread::sleep(std::time::Duration::from_millis(10));
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to execute parallel queries: {}", e);

                    if let Ok(mut state) = state_arc.try_write() {
                        state.loading_tasks.remove(&cache_key);
                        state.reset_phase1_state();
                    }
                }
            }
        });
    }

    /// Trigger selective refresh for selected combinations
    fn trigger_selective_refresh(&self, ctx: &Context) {
        let selected_combinations: Vec<String> = self
            .refresh_selection
            .iter()
            .filter_map(|(combo, &selected)| if selected { Some(combo.clone()) } else { None })
            .collect();

        if !selected_combinations.is_empty() {
            // Clone state for async operation
            let state_arc = self.state.clone();

            // Remove selected combinations from cache to force refresh
            // Map display names to cache keys using the stored mapping
            let shared_cache = super::cache::shared_cache();
            if let Ok(mut state) = self.state.try_write() {
                for display_name in &selected_combinations {
                    if let Some(cache_key) = self.refresh_display_to_cache.get(display_name) {
                        // Remove from both shared cache and state.cached_queries
                        shared_cache.remove_resources(cache_key);
                        state.cached_queries.remove(cache_key);
                        tracing::info!(
                            "Cleared cache for combination: {} (display: {})",
                            cache_key,
                            display_name
                        );
                    } else {
                        tracing::warn!(
                            "No cache key mapping found for display name: {}",
                            display_name
                        );
                    }
                }
            }

            let scope = if let Ok(state) = self.state.try_read() {
                state.query_scope.clone()
            } else {
                return;
            };

            // Use shared cache instead of state.cached_queries
            let cache = super::cache::shared_cache();

            // Clone AWS client for thread
            let aws_client = match &self.aws_client {
                Some(client) => client.clone(),
                None => {
                    warn!("AWS client not available for refresh - AWS Identity Center may not be configured");
                    return;
                }
            };

            // Mark as loading and request UI repaint
            let cache_key = format!("refresh_{}", chrono::Utc::now().timestamp_millis());
            if let Ok(mut loading_state) = self.state.try_write() {
                loading_state.loading_tasks.insert(cache_key.clone());
            }

            // Force UI repaint to show spinner immediately and schedule continuous updates
            ctx.request_repaint_after(std::time::Duration::from_millis(50));

            // Spawn background thread to avoid blocking UI
            std::thread::spawn(move || {
                // Create tokio runtime for async operations (following aws_identity pattern)
                let runtime = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(e) => {
                        tracing::error!("Failed to create Tokio runtime: {}", e);
                        // Remove loading indicator
                        if let Ok(mut state) = state_arc.try_write() {
                            state.loading_tasks.remove(&cache_key);
                        }
                        return;
                    }
                };

                // Perform parallel refresh queries with real-time updates
                type RefreshResult = Result<
                    (
                        Vec<super::state::ResourceEntry>,
                        HashMap<String, Vec<super::state::ResourceEntry>>,
                    ),
                    anyhow::Error,
                >;
                let result: RefreshResult = runtime.block_on(async {
                    // Use shared cache directly (already Arc<SharedResourceCache>)
                    let cache_arc = cache.clone();

                    // Create channels for parallel results
                    let (result_sender, mut result_receiver) =
                        tokio::sync::mpsc::channel::<super::aws_client::QueryResult>(1000);
                    let (progress_sender, mut progress_receiver) =
                        tokio::sync::mpsc::channel::<super::aws_client::QueryProgress>(100);

                    // Clone data for async tasks to avoid lifetime issues
                    let aws_client_clone = aws_client.clone();
                    let scope_clone = scope.clone();
                    let cache_clone = cache_arc.clone();

                    // Start parallel queries
                    let query_future = aws_client_clone.query_aws_resources_parallel(
                        &scope_clone,
                        result_sender,
                        Some(progress_sender),
                        cache_clone,
                    );

                    // Use Arc<Mutex> to share resources between async tasks for refresh
                    let all_resources = Arc::new(tokio::sync::Mutex::new(Vec::new()));
                    let all_resources_clone = all_resources.clone();
                    let state_arc_clone = state_arc.clone();

                    // Run query and result processing concurrently for refresh
                    let result_processing = async {
                        while let Some(result) = result_receiver.recv().await {
                            match result.resources {
                                Ok(resources) => {
                                    tracing::info!(
                                        "Refreshed {} resources for {}:{}:{}",
                                        resources.len(),
                                        result.account_id,
                                        result.region,
                                        result.resource_type
                                    );

                                    // Add to shared collection
                                    {
                                        let mut all_res = all_resources_clone.lock().await;
                                        all_res.extend(resources);

                                        // Update UI in real-time
                                        if let Ok(mut state) = state_arc_clone.try_write() {
                                            state.resources = all_res.clone();
                                        }
                                    }
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "Refresh failed for {}:{}:{}: {}",
                                        result.account_id,
                                        result.region,
                                        result.resource_type,
                                        e
                                    );
                                }
                            }
                        }
                        let final_count = all_resources_clone.lock().await.len();
                        tracing::debug!(
                            "Refresh result receiver channel closed, collected {} total resources",
                            final_count
                        );
                    };

                    // Process progress updates concurrently
                    let progress_processing = async {
                        while let Some(progress) = progress_receiver.recv().await {
                            tracing::debug!(
                                "Refresh progress: {} - {} - {}",
                                progress.account,
                                progress.region,
                                progress.message
                            );
                        }
                        tracing::debug!("Refresh progress receiver channel closed");
                    };

                    // Run ALL THREE concurrently for refresh - this is the key fix!
                    tokio::join!(
                        async {
                            match query_future.await {
                                Ok(()) => tracing::debug!(
                                    "Refresh query execution completed successfully"
                                ),
                                Err(e) => {
                                    tracing::error!("Parallel refresh execution failed: {}", e)
                                }
                            }
                        },
                        result_processing,
                        progress_processing
                    );

                    // Get final results from shared storage
                    let final_resources = all_resources.lock().await.clone();

                    // Return final cache and resources (convert from shared cache for compatibility)
                    let final_cache = cache_arc.to_hashmap();
                    Ok((final_resources, final_cache))
                });

                // Update state with results
                match result {
                    Ok((resources, final_cache)) => {
                        // Retry logic to ensure we update state even if locked
                        // UI thread only holds lock for ~1-5ms per frame (60fps = 16ms between frames)
                        // Retry for up to 3 seconds to be absolutely sure
                        let max_attempts = 30;
                        let retry_delay_ms = 100;

                        for attempt in 0..max_attempts {
                            if let Ok(mut state) = state_arc.try_write() {
                                state.cached_queries = final_cache.clone();

                                // Apply the same filtering as initial query to ensure consistent behavior
                                // S3 buckets are hybrid-global: queried globally but filtered by actual region
                                let mut filtered_resources = Vec::new();
                                for resource in &resources {
                                    // Check account match
                                    let account_matches = scope.accounts.iter()
                                        .any(|a| a.account_id == resource.account_id);

                                    // True global resources match any region; S3 buckets filtered by actual region
                                    let is_true_global = resource.region == "Global"
                                        && resource.resource_type != "AWS::S3::Bucket";
                                    let region_matches = is_true_global
                                        || scope.regions.iter().any(|r| r.region_code == resource.region);

                                    // Check resource type match
                                    let type_matches = scope.resource_types.iter()
                                        .any(|rt| rt.resource_type == resource.resource_type);

                                    if account_matches && region_matches && type_matches {
                                        filtered_resources.push(resource.clone());
                                    }
                                }
                                state.resources = filtered_resources;
                                state.loading_tasks.remove(&cache_key);

                                tracing::info!(
                                    "Successfully refreshed {} combinations with {} resources (attempt {})",
                                    selected_combinations.len(),
                                    state.resources.len(),
                                    attempt + 1
                                );
                                break;
                            } else if attempt < max_attempts - 1 {
                                if attempt == 0 {
                                    tracing::debug!(
                                        "State locked, retrying to update after refresh..."
                                    );
                                } else if attempt % 10 == 0 {
                                    tracing::warn!(
                                        "Still retrying to acquire state lock (attempt {})",
                                        attempt + 1
                                    );
                                }
                                std::thread::sleep(std::time::Duration::from_millis(
                                    retry_delay_ms,
                                ));
                            } else {
                                tracing::error!("CRITICAL: Failed to update state after refresh after {} attempts ({}s total)",
                                    max_attempts, max_attempts * retry_delay_ms / 1000);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to refresh AWS resources: {}", e);

                        // Remove loading indicator with retry (critical to prevent stuck spinner)
                        for attempt in 0..30 {
                            if let Ok(mut state) = state_arc.try_write() {
                                state.loading_tasks.remove(&cache_key);
                                tracing::info!(
                                    "Cleared loading task after error (attempt {})",
                                    attempt + 1
                                );
                                break;
                            } else if attempt < 29 {
                                std::thread::sleep(std::time::Duration::from_millis(100));
                            } else {
                                tracing::error!("CRITICAL: Failed to clear loading task after error - spinner may be stuck");
                            }
                        }
                    }
                }
            });
        }
    }

    fn handle_active_selection_reduction(
        &self,
        state: &mut ResourceExplorerState,
        was_phase2_running: bool,
    ) {
        state.cancel_phase2_enrichment();
        self.filter_resources_by_current_scope(state);

        if was_phase2_running {
            self.maybe_start_phase2_enrichment(state);
        }
    }

    fn resource_matches_scope(resource: &ResourceEntry, scope: &QueryScope) -> bool {
        let account_matches = scope
            .accounts
            .iter()
            .any(|a| a.account_id == resource.account_id);

        // True global resources (IAM, Route53, etc.) match any region in the scope.
        // S3 buckets are hybrid-global: queried globally but have actual regions.
        // They are filtered by their actual bucket region to only show buckets
        // in selected regions (user expectation: select us-east-1, see us-east-1 buckets).
        let is_true_global_resource = resource.region == "Global"
            && resource.resource_type != "AWS::S3::Bucket";

        let region_matches = is_true_global_resource
            || scope
                .regions
                .iter()
                .any(|r| r.region_code == resource.region);

        let resource_type_matches = scope
            .resource_types
            .iter()
            .any(|rt| rt.resource_type == resource.resource_type);

        account_matches && region_matches && resource_type_matches
    }

    fn refresh_resources_from_cache_filtered(
        state: &mut ResourceExplorerState,
        cache: &HashMap<String, Vec<ResourceEntry>>,
    ) {
        let mut refreshed_resources = Vec::new();

        for cached_entries in cache.values() {
            for resource in cached_entries {
                if Self::resource_matches_scope(resource, &state.query_scope) {
                    refreshed_resources.push(resource.clone());
                }
            }
        }

        state.resources = refreshed_resources;
    }

    /// Sync detailed_properties from cache to existing resources in-place.
    /// This does NOT trigger a tree rebuild - only updates property data.
    /// Used during Phase 2 progress updates to avoid UI disruption.
    fn sync_detailed_properties_from_cache(
        state: &mut ResourceExplorerState,
        cache: &HashMap<String, Vec<ResourceEntry>>,
    ) {
        // Build a lookup map from cache for O(1) access
        let mut cache_lookup: HashMap<(String, String, String, String), &ResourceEntry> =
            HashMap::new();
        for entries in cache.values() {
            for entry in entries {
                let key = (
                    entry.account_id.clone(),
                    entry.region.clone(),
                    entry.resource_id.clone(),
                    entry.resource_type.clone(),
                );
                cache_lookup.insert(key, entry);
            }
        }

        // Update detailed_properties in-place on existing resources
        for resource in &mut state.resources {
            let key = (
                resource.account_id.clone(),
                resource.region.clone(),
                resource.resource_id.clone(),
                resource.resource_type.clone(),
            );
            if let Some(cached) = cache_lookup.get(&key) {
                if cached.detailed_properties.is_some() && resource.detailed_properties.is_none() {
                    resource.detailed_properties = cached.detailed_properties.clone();
                    resource.detailed_timestamp = cached.detailed_timestamp;
                }
            }
        }
    }

    fn maybe_start_phase2_enrichment(&self, state: &mut ResourceExplorerState) {
        let Some(aws_client) = self.aws_client.clone() else {
            return;
        };
        let state_arc_for_phase2 = self.state.clone();
        Self::maybe_start_phase2_enrichment_for_state(state, state_arc_for_phase2, aws_client);
    }

    fn maybe_start_phase2_enrichment_for_state(
        state: &mut ResourceExplorerState,
        state_arc_for_phase2: Arc<RwLock<ResourceExplorerState>>,
        aws_client: Arc<AWSResourceClient>,
    ) {
        // Sync detailed_properties from cache BEFORE checking what needs enrichment
        // This ensures we recognize resources that were enriched in a previous interrupted Phase 2
        let cache_clone = state.cached_queries.clone();
        Self::sync_detailed_properties_from_cache(state, &cache_clone);

        let enrichable_types = super::state::ResourceExplorerState::enrichable_resource_types();

        // Only enrich resources that are visible (in state.resources, filtered by selected regions)
        let resources_to_enrich: Vec<_> = state
            .resources
            .iter()
            .filter(|r| {
                enrichable_types.contains(&r.resource_type.as_str())
                    && r.detailed_properties.is_none()
            })
            .cloned()
            .collect();

        if resources_to_enrich.is_empty() {
            return;
        }

        if state.phase2_enrichment_in_progress {
            state.cancel_phase2_enrichment();
        }

        state.phase2_enrichment_in_progress = true;
        state.phase2_enrichment_completed = false;
        state.phase2_progress_total = resources_to_enrich.len();
        state.phase2_progress_count = 0;

        let phase2_generation = state.phase2_generation;
        // Use shared cache for Phase 2 enrichment
        let cache_for_phase2 = super::cache::shared_cache();

        // Debug: log cache keys and resources at Phase 2 start
        let cache_keys = cache_for_phase2.resource_keys();
        let s3_resources_with_details: usize = state.resources.iter()
            .filter(|r| r.resource_type == "AWS::S3::Bucket" && r.detailed_properties.is_some())
            .count();

        tracing::info!(
            "🔄 Starting Phase 2 enrichment for {} resources (cache keys: {:?}, S3 with details: {})",
            resources_to_enrich.len(),
            cache_keys,
            s3_resources_with_details
        );

        std::thread::spawn(move || {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    tracing::error!("Failed to create Phase 2 runtime: {}", e);
                    if let Ok(mut s) = state_arc_for_phase2.try_write() {
                        s.phase2_enrichment_in_progress = false;
                    }
                    return;
                }
            };

            rt.block_on(async {
                let (progress_tx, mut progress_rx) =
                    tokio::sync::mpsc::channel::<super::aws_client::QueryProgress>(100);
                let (result_tx, _result_rx) =
                    tokio::sync::mpsc::channel::<super::aws_client::QueryResult>(100);

                let cache_for_sync = cache_for_phase2.clone();

                aws_client.start_phase2_enrichment(
                    resources_to_enrich,
                    result_tx,
                    Some(progress_tx),
                    cache_for_phase2.clone(),
                );

                let mut canceled = false;

                // Debounce resource refresh to reduce UI flickering (2 second interval)
                // Status bar updates happen on every message, but tree data refresh is debounced
                let resource_refresh_debounce_ms: u128 = 2000;
                let mut last_resource_refresh = std::time::Instant::now();

                while let Some(progress) = progress_rx.recv().await {
                    let generation_matches = {
                        let s = state_arc_for_phase2.read().await;
                        s.phase2_generation == phase2_generation
                    };

                    if !generation_matches {
                        canceled = true;
                        tracing::info!(
                            "Phase 2 enrichment canceled due to active selection change"
                        );
                        break;
                    }

                    let is_completion = matches!(
                        progress.status,
                        super::aws_client::QueryStatus::EnrichmentCompleted
                    );
                    let is_enrichment_update = matches!(
                        progress.status,
                        super::aws_client::QueryStatus::EnrichmentInProgress
                            | super::aws_client::QueryStatus::EnrichmentCompleted
                    );

                    let updated_cache = if is_enrichment_update {
                        // Convert from shared cache to HashMap for compatibility
                        Some(cache_for_sync.to_hashmap())
                    } else {
                        None
                    };

                    if is_completion {
                        let mut s = state_arc_for_phase2.write().await;

                        s.phase2_current_service = Some(progress.resource_type.clone());
                        if let (Some(processed), Some(total)) =
                            (progress.items_processed, progress.estimated_total)
                        {
                            s.phase2_progress_count = processed;
                            s.phase2_progress_total = total;
                        }

                        if let Some(cache) = updated_cache {
                            // Debug: Check cache before sync
                            let s3_cache_key = cache.keys().find(|k| k.contains("S3::Bucket"));
                            let s3_with_details_in_cache = s3_cache_key.map(|key| {
                                cache.get(key).map(|resources| {
                                    resources.iter().filter(|r| r.detailed_properties.is_some()).count()
                                }).unwrap_or(0)
                            }).unwrap_or(0);
                            tracing::debug!(
                                "Phase 2 sync: S3 cache key: {:?}, resources with details: {}",
                                s3_cache_key,
                                s3_with_details_in_cache
                            );

                            s.cached_queries = cache;
                            let cached_queries = s.cached_queries.clone();
                            Self::refresh_resources_from_cache_filtered(&mut s, &cached_queries);
                            // Force tree rebuild at completion to show final data
                            s.increment_enrichment_version_force();

                            // Debug: Check state.resources after sync
                            let s3_with_details_in_state = s.resources.iter()
                                .filter(|r| r.resource_type == "AWS::S3::Bucket" && r.detailed_properties.is_some())
                                .count();
                            tracing::info!(
                                "✅ Phase 2 enrichment completed: {} S3 buckets with detailed_properties in state.resources",
                                s3_with_details_in_state
                            );

                            // Debug: Check state.resources after sync
                            let s3_with_details_in_state = s.resources.iter()
                                .filter(|r| r.resource_type == "AWS::S3::Bucket" && r.detailed_properties.is_some())
                                .count();
                            tracing::info!(
                                "✅ Phase 2 enrichment completed: {} S3 buckets with detailed_properties in state.resources",
                                s3_with_details_in_state
                            );

                            s.phase2_enrichment_in_progress = false;
                            s.phase2_enrichment_completed = true;
                            s.phase2_current_service = None;
                            tracing::info!(
                                "✅ Phase 2 enrichment completed, synced {} resources to UI",
                                s.resources.len()
                            );
                        }

                        break;
                    } else if let Ok(mut s) = state_arc_for_phase2.try_write() {
                        // Always update status bar progress (fast updates)
                        s.phase2_current_service = Some(progress.resource_type.clone());
                        if let (Some(processed), Some(total)) =
                            (progress.items_processed, progress.estimated_total)
                        {
                            s.phase2_progress_count = processed;
                            s.phase2_progress_total = total;
                        }

                        // Debounce resource refresh to reduce UI flickering
                        // Only sync cache to resources every 2 seconds during enrichment
                        let elapsed_since_refresh = last_resource_refresh.elapsed().as_millis();
                        if elapsed_since_refresh >= resource_refresh_debounce_ms {
                            if let Some(cache) = updated_cache {
                                s.cached_queries = cache;
                                let cached_queries = s.cached_queries.clone();
                                // Use efficient in-place sync that only updates detailed_properties
                                // This is faster than full refresh and doesn't trigger tree rebuild
                                Self::sync_detailed_properties_from_cache(&mut s, &cached_queries);
                            }
                            last_resource_refresh = std::time::Instant::now();
                        }
                    }
                }

                if canceled {
                    return;
                }

                let generation_matches = {
                    let s = state_arc_for_phase2.read().await;
                    s.phase2_generation == phase2_generation
                };

                if !generation_matches {
                    return;
                }

                // Convert from shared cache to HashMap for compatibility
                let updated_cache = cache_for_sync.to_hashmap();

                let mut s = state_arc_for_phase2.write().await;
                if s.phase2_enrichment_in_progress {
                    s.cached_queries = updated_cache;
                    let cached_queries = s.cached_queries.clone();
                    Self::refresh_resources_from_cache_filtered(&mut s, &cached_queries);
                    // Force tree rebuild at cleanup to show final data
                    s.increment_enrichment_version_force();

                    s.phase2_enrichment_in_progress = false;
                    s.phase2_enrichment_completed = true;
                    s.phase2_current_service = None;
                    tracing::info!(
                        "✅ Phase 2 cleanup: marked enrichment complete after channel close"
                    );
                }
            });
        });
    }

    /// Filter displayed resources to match current query scope without clearing cache
    /// This preserves cached data while updating what's visible in the tree
    fn filter_resources_by_current_scope(&self, state: &mut ResourceExplorerState) {
        let mut filtered_resources = Vec::new();

        for resource in &state.resources {
            if Self::resource_matches_scope(resource, &state.query_scope) {
                filtered_resources.push(resource.clone());
            }
        }

        state.resources = filtered_resources;

        tracing::info!(
            "Filtered displayed resources to {} items matching current scope",
            state.resources.len()
        );
    }

    /// Get available AWS accounts from Identity Center, fallback to default if not available
    fn get_available_accounts(&self) -> Vec<crate::app::aws_identity::AwsAccount> {
        if let Some(ref identity_center) = self.aws_identity_center {
            if let Ok(identity) = identity_center.lock() {
                // Get real accounts from AWS Identity Center
                let real_accounts: Vec<crate::app::aws_identity::AwsAccount> =
                    identity.accounts.to_vec();

                if !real_accounts.is_empty() {
                    // Only log account retrieval once per session to avoid flooding
                    tracing::debug!(
                        "Retrieved {} real AWS accounts from Identity Center",
                        real_accounts.len()
                    );
                    return real_accounts;
                }
            }
        }

        // Fallback: warn and return empty list instead of fake accounts
        tracing::warn!("No real AWS accounts available - AWS Identity Center may not be logged in");
        Vec::new()
    }
}
