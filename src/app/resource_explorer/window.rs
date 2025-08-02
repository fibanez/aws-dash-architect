use super::{aws_client::*, colors::*, dialogs::*, state::*, tree::*};
use crate::app::aws_identity::AwsIdentityCenter;
use egui::{Color32, Context, Ui, Window};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
use tracing::warn;

pub struct ResourceExplorerWindow {
    state: Arc<RwLock<ResourceExplorerState>>,
    is_open: bool,
    is_focused: bool,
    fuzzy_dialog: FuzzySearchDialog,
    tree_renderer: TreeRenderer,
    aws_client: Option<AWSResourceClient>,
    refresh_selection: HashMap<String, bool>, // Track which combinations to refresh
    show_refresh_dialog: bool,                // Local dialog state to avoid borrow conflicts
    aws_identity_center: Option<Arc<Mutex<AwsIdentityCenter>>>, // Access to real AWS accounts
    failed_detail_requests: Arc<tokio::sync::RwLock<std::collections::HashSet<String>>>, // Track failed requests
}

impl ResourceExplorerWindow {
    pub fn new(state: Arc<RwLock<ResourceExplorerState>>) -> Self {
        Self {
            state,
            is_open: false,
            is_focused: false,
            fuzzy_dialog: FuzzySearchDialog::new(),
            tree_renderer: TreeRenderer::new(),
            aws_client: None,
            refresh_selection: HashMap::new(),
            show_refresh_dialog: false,
            aws_identity_center: None,
            failed_detail_requests: Arc::new(tokio::sync::RwLock::new(
                std::collections::HashSet::new(),
            )),
        }
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

                // Create credential coordinator with live reference to identity center
                let credential_coordinator = Arc::new(
                    crate::app::resource_explorer::credentials::CredentialCoordinator::new(
                        identity_center_mutex.clone(),
                        default_role,
                    ),
                );

                // Create AWS client with credential coordinator
                self.aws_client = Some(AWSResourceClient::new(credential_coordinator));
            }
        } else {
            // Clear AWS client if identity center is removed
            self.aws_client = None;
        }
    }

    /// Get reference to the AWS client for use by other components
    pub fn get_aws_client(&self) -> Option<Arc<AWSResourceClient>> {
        self.aws_client
            .as_ref()
            .map(|client| Arc::new(client.clone()))
    }

    pub fn show(&mut self, ctx: &Context) -> bool {
        if !self.is_open {
            return false;
        }

        // Request continuous repaints if we have active loading tasks to show spinner animation
        if let Ok(state) = self.state.try_read() {
            if state.is_loading() {
                // Request repaint every 100ms to keep spinner animated
                ctx.request_repaint_after(std::time::Duration::from_millis(100));
            }
        }

        let mut is_open = self.is_open;

        // Calculate window size based on screen dimensions
        let screen_rect = ctx.screen_rect();
        let top_bar_height = 60.0; // Approximate height of the top menu bar
        let window_width = 800.0;
        let window_height = screen_rect.height() - top_bar_height - 40.0; // Leave some margin

        let response = Window::new("AWS Explorer")
            .open(&mut is_open)
            .default_size([window_width, window_height])
            .min_size([600.0, 400.0]) // Set minimum size to prevent window from getting too small
            .resizable(true)
            .show(ctx, |ui| {
                // Check if this window has focus
                self.is_focused = ui
                    .ctx()
                    .memory(|mem| mem.focused().map(|id| id == ui.id()).unwrap_or(false));

                if let Ok(mut state) = self.state.try_write() {
                    Self::render_toolbar_static(ui, &mut state);
                    ui.separator();
                    self.render_active_tags_static(ui, &mut state);
                    ui.separator();
                    Self::render_search_bar_static(ui, &mut state);
                    ui.separator();
                    Self::render_grouping_controls_static(ui, &mut state);
                    ui.separator();
                    Self::render_tree_view_static(ui, &state, &mut self.tree_renderer);
                } else {
                    ui.label("Loading...");
                }
            });

        // Update is_open from the window response
        self.is_open = is_open;

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
                self.process_pending_detail_requests(&state, ctx, requests);
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
        }

        if self.show_refresh_dialog {
            self.render_refresh_dialog_standalone(ctx);
        }

        response.is_some()
    }

    fn render_toolbar_static(ui: &mut Ui, state: &mut ResourceExplorerState) {
        ui.horizontal(|ui| {
            if ui.button("Add Account").clicked() {
                state.show_account_dialog = true;
            }

            if ui.button("Add Region").clicked() {
                state.show_region_dialog = true;
            }

            if ui.button("Add Resource").clicked() {
                state.show_resource_type_dialog = true;
            }

            ui.separator();

            if ui.button("Refresh").clicked() {
                state.show_refresh_dialog = true;
            }

            // Show loading indicator if queries are active
            if state.is_loading() {
                ui.separator();
                ui.spinner();
                ui.label(format!(
                    "Loading... ({} queries)",
                    state.loading_tasks.len()
                ));
            }
        });
    }

    fn render_active_tags_static(&self, ui: &mut Ui, state: &mut ResourceExplorerState) {
        ui.label("Active Selection:");

        ui.horizontal_wrapped(|ui| {
            // Account tags with new colored tag rendering
            let mut accounts_to_remove = Vec::new();
            for account in &state.query_scope.accounts {
                if Self::render_closeable_account_tag(ui, &account.account_id, &account.display_name, account.color) {
                    accounts_to_remove.push(account.account_id.clone());
                }
                ui.add_space(4.0);
            }
            for account_id in accounts_to_remove {
                tracing::info!("❌ Removing account: {}", account_id);
                state.remove_account(&account_id);

                // Log current scope after removal
                tracing::info!("📊 Current scope after removing account: {} accounts, {} regions, {} resource types",
                    state.query_scope.accounts.len(),
                    state.query_scope.regions.len(),
                    state.query_scope.resource_types.len());

                // Filter displayed resources when account is removed
                self.filter_resources_by_current_scope(state);
            }

            // Region tags with new colored tag rendering
            let mut regions_to_remove = Vec::new();
            for region in &state.query_scope.regions {
                if Self::render_closeable_region_tag(ui, &region.region_code, &region.display_name, region.color) {
                    regions_to_remove.push(region.region_code.clone());
                }
                ui.add_space(4.0);
            }
            for region_code in regions_to_remove {
                tracing::info!("❌ Removing region: {}", region_code);
                state.remove_region(&region_code);

                // Log current scope after removal
                tracing::info!("📊 Current scope after removing region: {} accounts, {} regions, {} resource types",
                    state.query_scope.accounts.len(),
                    state.query_scope.regions.len(),
                    state.query_scope.resource_types.len());

                // Filter displayed resources when region is removed
                self.filter_resources_by_current_scope(state);
            }

            // Resource type tags with improved styling
            let mut resource_types_to_remove = Vec::new();
            for resource_type in &state.query_scope.resource_types {
                // Count resources for this resource type
                let resource_count = state.resources.iter()
                    .filter(|r| r.resource_type == resource_type.resource_type)
                    .count();

                if Self::render_closeable_resource_type_tag_with_count(ui, &resource_type.resource_type, &resource_type.display_name, resource_count) {
                    resource_types_to_remove.push(resource_type.resource_type.clone());
                }
                ui.add_space(4.0);
            }
            for resource_type in resource_types_to_remove {
                tracing::info!("❌ Removing resource type: {}", resource_type);
                state.remove_resource_type(&resource_type);

                // Log current scope after removal
                tracing::info!("📊 Current scope after removing resource type: {} accounts, {} regions, {} resource types",
                    state.query_scope.accounts.len(),
                    state.query_scope.regions.len(),
                    state.query_scope.resource_types.len());

                // Filter displayed resources when resource type is removed
                self.filter_resources_by_current_scope(state);
            }
        });
    }

    fn render_search_bar_static(ui: &mut Ui, state: &mut ResourceExplorerState) {
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut state.search_filter);
            if ui.button("Clear").clicked() {
                state.search_filter.clear();
            }
            ui.label("🔍");
        });
    }

    fn render_grouping_controls_static(ui: &mut Ui, state: &mut ResourceExplorerState) {
        ui.horizontal(|ui| {
            ui.label("Group by:");

            // Primary grouping dropdown
            egui::ComboBox::from_label("")
                .selected_text(state.primary_grouping.display_name())
                .show_ui(ui, |ui| {
                    for mode in GroupingMode::all_modes() {
                        ui.selectable_value(
                            &mut state.primary_grouping,
                            mode.clone(),
                            mode.display_name(),
                        );
                    }
                });
        });
    }

    fn render_tree_view_static(
        ui: &mut Ui,
        state: &ResourceExplorerState,
        tree_renderer: &mut TreeRenderer,
    ) {
        // Use remaining available space for the tree view with scrolling
        egui::ScrollArea::vertical()
            .auto_shrink([false, false]) // Don't shrink the scroll area based on content
            .show(ui, |ui| {
                if state.query_scope.is_empty() {
                    ui.centered_and_justified(|ui| {
                        ui.label("Select accounts, regions, and resource types to begin exploring");
                    });
                } else if state.resources.is_empty() && !state.is_loading() {
                    ui.centered_and_justified(|ui| {
                        ui.label("No resources found for the current selection");
                    });
                } else if !state.resources.is_empty() {
                    // Use cached tree rendering to prevent unnecessary rebuilds
                    tree_renderer.render_tree_cached(
                        ui,
                        &state.resources,
                        state.primary_grouping.clone(),
                        &state.search_filter,
                    );
                } else if state.is_loading() {
                    ui.centered_and_justified(|ui| {
                        ui.spinner();
                        ui.label("Loading resources...");
                    });
                }
            });
    }

    fn render_refresh_dialog_standalone(&mut self, ctx: &Context) {
        if !self.show_refresh_dialog {
            return;
        }

        let combinations = if let Ok(state) = self.state.try_read() {
            self.generate_refresh_combinations(&state)
        } else {
            Vec::new()
        };

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
                    // Also clear the state flag
                    if let Ok(mut state) = self.state.try_write() {
                        state.show_refresh_dialog = false;
                    }
                } else if refresh_clicked {
                    // Trigger refresh for selected combinations
                    self.trigger_selective_refresh(ctx);
                    self.show_refresh_dialog = false;
                    self.refresh_selection.clear();
                    // Also clear the state flag
                    if let Ok(mut state) = self.state.try_write() {
                        state.show_refresh_dialog = false;
                    }
                }
            }
        }
    }

    fn generate_refresh_combinations(&self, state: &ResourceExplorerState) -> Vec<String> {
        let mut combinations = Vec::new();

        for account in &state.query_scope.accounts {
            for region in &state.query_scope.regions {
                for resource_type in &state.query_scope.resource_types {
                    combinations.push(format!(
                        "{}/{}/{}",
                        account.display_name, region.display_name, resource_type.display_name
                    ));
                }
            }
        }

        combinations
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
            let cache = Arc::new(tokio::sync::RwLock::new(state.cached_queries.clone()));

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

            // Spawn background thread to avoid blocking UI
            std::thread::spawn(move || {
                // Create tokio runtime for async operations
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

                // Perform parallel queries with real-time updates
                let result: Result<Vec<super::state::ResourceEntry>, anyhow::Error> = runtime
                    .block_on(async {
                        // Create channels for parallel results
                        let (result_sender, mut result_receiver) =
                            tokio::sync::mpsc::channel::<super::aws_client::QueryResult>(1000);
                        let (progress_sender, mut progress_receiver) =
                            tokio::sync::mpsc::channel::<super::aws_client::QueryProgress>(100);

                        // Clone data for spawned tasks to avoid lifetime issues
                        let aws_client_clone = aws_client.clone();
                        let scope_clone = scope.clone();
                        let cache_clone = cache.clone();

                        // Start parallel queries
                        let query_future = aws_client_clone.query_aws_resources_parallel(
                            &scope_clone,
                            result_sender,
                            Some(progress_sender),
                            cache_clone,
                        );

                        // Use Arc<Mutex> to share resources between async tasks
                        let all_resources = Arc::new(tokio::sync::Mutex::new(Vec::new()));
                        let all_resources_clone = all_resources.clone();
                        let state_arc_clone = state_arc.clone();

                        // Start query future - no need to spawn since we're not moving across thread boundaries
                        // let query_handle = tokio::spawn(query_future);

                        // Run query and result processing concurrently - the key is CONCURRENT not sequential
                        let result_processing = async {
                            while let Some(result) = result_receiver.recv().await {
                                match result.resources {
                                    Ok(resources) => {
                                        tracing::info!(
                                            "Received {} resources for {}:{}:{}",
                                            resources.len(),
                                            result.account_id,
                                            result.region,
                                            result.resource_type
                                        );

                                        // Add to shared collection
                                        {
                                            let mut all_res = all_resources_clone.lock().await;
                                            all_res.extend(resources);

                                            // Update UI in real-time with current resources
                                            if let Ok(mut state) = state_arc_clone.try_write() {
                                                state.resources = all_res.clone();
                                                // Note: Don't remove loading tasks yet, we're still receiving results
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        tracing::error!(
                                            "Query failed for {}:{}:{}: {}",
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
                                "Result receiver channel closed, collected {} total resources",
                                final_count
                            );
                        };

                        // Process progress updates concurrently
                        let progress_processing = async {
                            while let Some(progress) = progress_receiver.recv().await {
                                tracing::debug!(
                                    "Progress: {} - {} - {}",
                                    progress.account,
                                    progress.region,
                                    progress.message
                                );
                            }
                            tracing::debug!("Progress receiver channel closed");
                        };

                        // Run ALL THREE concurrently - this is the key fix!
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

                        // Get final results from shared storage
                        let final_resources = all_resources.lock().await.clone();

                        Ok(final_resources)
                    });

                // Final update with all results
                match result {
                    Ok(resources) => {
                        // Update state with results and remove loading indicator - use retry with fallback
                        for attempt in 0..3 {
                            if let Ok(mut state) = state_arc.try_write() {
                                // Update cached queries from the async cache
                                let final_cache =
                                    runtime.block_on(async { cache.read().await.clone() });
                                state.cached_queries = final_cache;
                                state.resources = resources;
                                state.finish_loading_task(&cache_key);

                                tracing::info!("✅ Parallel query completed: {} total resources (loading tasks remaining: {})",
                                    state.resources.len(), state.loading_task_count());
                                break;
                            } else if attempt == 2 {
                                tracing::warn!("Failed to update state after query completion after 3 attempts");
                            } else {
                                std::thread::sleep(std::time::Duration::from_millis(10));
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to execute parallel queries: {}", e);

                        // Remove loading indicator
                        if let Ok(mut state) = state_arc.try_write() {
                            state.loading_tasks.remove(&cache_key);
                        }
                    }
                }
            });
        }
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
            if let Ok(mut state) = self.state.try_write() {
                for combo in &selected_combinations {
                    // Parse combination string to extract account, region, resource type
                    let parts: Vec<&str> = combo.split('/').collect();
                    if parts.len() == 3 {
                        let cache_key = format!("{}:{}:{}", parts[0], parts[1], parts[2]);
                        state.cached_queries.remove(&cache_key);
                        tracing::info!("Cleared cache for combination: {}", cache_key);
                    }
                }
            }

            let scope = if let Ok(state) = self.state.try_read() {
                state.query_scope.clone()
            } else {
                return;
            };

            let cache = if let Ok(state) = self.state.try_read() {
                state.cached_queries.clone()
            } else {
                return;
            };

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
                    // Convert to Arc<RwLock> for parallel method
                    let cache_arc = Arc::new(tokio::sync::RwLock::new(cache));

                    // Create channels for parallel results
                    let (result_sender, mut result_receiver) =
                        tokio::sync::mpsc::channel::<super::aws_client::QueryResult>(1000);
                    let (progress_sender, mut progress_receiver) =
                        tokio::sync::mpsc::channel::<super::aws_client::QueryProgress>(100);

                    // Clone data for async tasks to avoid lifetime issues
                    let aws_client_clone = aws_client.clone();
                    let scope_clone = scope.clone();
                    let cache_arc_clone = cache_arc.clone();

                    // Start parallel queries
                    let query_future = aws_client_clone.query_aws_resources_parallel(
                        &scope_clone,
                        result_sender,
                        Some(progress_sender),
                        cache_arc_clone,
                    );

                    // Use Arc<Mutex> to share resources between async tasks for refresh
                    let all_resources = Arc::new(tokio::sync::Mutex::new(Vec::new()));
                    let all_resources_clone = all_resources.clone();
                    let state_arc_clone = state_arc.clone();

                    // Start query future - no need to spawn since we're not crossing thread boundaries
                    // let query_handle = tokio::spawn(query_future);

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

                    // Return final cache and resources
                    let final_cache = cache_arc.read().await.clone();
                    Ok((final_resources, final_cache))
                });

                // Update state with results
                match result {
                    Ok((resources, final_cache)) => {
                        if let Ok(mut state) = state_arc.try_write() {
                            state.resources = resources;
                            state.cached_queries = final_cache;
                            state.loading_tasks.remove(&cache_key);

                            tracing::info!(
                                "Successfully refreshed {} combinations with {} resources",
                                selected_combinations.len(),
                                state.resources.len()
                            );
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to refresh AWS resources: {}", e);

                        // Remove loading indicator
                        if let Ok(mut state) = state_arc.try_write() {
                            state.loading_tasks.remove(&cache_key);
                        }
                    }
                }
            });
        }
    }

    /// Filter displayed resources to match current query scope without clearing cache
    /// This preserves cached data while updating what's visible in the tree
    fn filter_resources_by_current_scope(&self, state: &mut ResourceExplorerState) {
        // Keep all cached resources intact
        let mut filtered_resources = Vec::new();

        // Only include resources that match current scope (accounts, regions, resource types)
        for resource in &state.resources {
            let account_matches = state
                .query_scope
                .accounts
                .iter()
                .any(|a| a.account_id == resource.account_id);

            let region_matches = state
                .query_scope
                .regions
                .iter()
                .any(|r| r.region_code == resource.region);

            let resource_type_matches = state
                .query_scope
                .resource_types
                .iter()
                .any(|rt| rt.resource_type == resource.resource_type);

            // Only include if ALL criteria match current scope
            if account_matches && region_matches && resource_type_matches {
                filtered_resources.push(resource.clone());
            }
        }

        // Update displayed resources (but preserve cache)
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

    /// Render a closeable account tag with colored background
    /// Returns true if the tag was clicked (should be removed)
    fn render_closeable_account_tag(
        ui: &mut Ui,
        account_id: &str,
        display_name: &str,
        tag_color: Color32,
    ) -> bool {
        let text_color = get_contrasting_text_color(tag_color);

        // Extract account name and ID from display_name, excluding email
        // Format is: "account_name - account_id (email)" -> we want "account_name - account_id"
        let clean_display_name = if let Some(email_start) = display_name.find(" (") {
            display_name[..email_start].to_string()
        } else {
            display_name.to_string()
        };

        // Create tag content with close button
        let tag_text = format!("{} ×", clean_display_name);

        // Calculate text size
        let font_size = 11.0;
        let text_galley = ui.fonts(|fonts| {
            fonts.layout_no_wrap(
                tag_text.clone(),
                egui::FontId::proportional(font_size),
                text_color,
            )
        });

        // Add padding to the text size
        let padding = egui::vec2(8.0, 4.0);
        let desired_size = text_galley.size() + 2.0 * padding;

        // Allocate space for the tag
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            // Draw rounded rectangle background
            ui.painter().rect_filled(
                rect, 4.0, // corner radius
                tag_color,
            );

            // Draw text centered in the rect
            let text_pos = rect.center() - text_galley.size() / 2.0;
            ui.painter().galley(text_pos, text_galley, text_color);
        }

        // Show tooltip on hover and return click status
        let clicked = response.clicked();
        response.on_hover_text(format!("Account: {} (click to remove)", account_id));
        clicked
    }

    /// Render a closeable region tag with colored background
    /// Returns true if the tag was clicked (should be removed)
    fn render_closeable_region_tag(
        ui: &mut Ui,
        region_code: &str,
        display_name: &str,
        tag_color: Color32,
    ) -> bool {
        let text_color = get_contrasting_text_color(tag_color);

        // Create tag content with close button
        let tag_text = format!("{} ×", display_name);

        // Calculate text size
        let font_size = 11.0;
        let text_galley = ui.fonts(|fonts| {
            fonts.layout_no_wrap(
                tag_text.clone(),
                egui::FontId::proportional(font_size),
                text_color,
            )
        });

        // Add padding to the text size
        let padding = egui::vec2(8.0, 4.0);
        let desired_size = text_galley.size() + 2.0 * padding;

        // Allocate space for the tag
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            // Draw rounded rectangle background
            ui.painter().rect_filled(
                rect, 4.0, // corner radius
                tag_color,
            );

            // Draw text centered in the rect
            let text_pos = rect.center() - text_galley.size() / 2.0;
            ui.painter().galley(text_pos, text_galley, text_color);
        }

        // Show tooltip on hover and return click status
        let clicked = response.clicked();
        response.on_hover_text(format!("Region: {} (click to remove)", region_code));
        clicked
    }

    /// Render a closeable resource type tag with count information
    /// Returns true if the tag was clicked (should be removed)
    fn render_closeable_resource_type_tag_with_count(
        ui: &mut Ui,
        resource_type: &str,
        display_name: &str,
        count: usize,
    ) -> bool {
        let tag_color = Color32::from_rgb(108, 117, 125); // Bootstrap secondary color
        let text_color = Color32::WHITE;

        // Create tag content with close button and count
        let tag_text = if count == 0 {
            format!("{} (0 Found) ×", display_name)
        } else {
            format!("{} ({}) ×", display_name, count)
        };

        // Calculate text size
        let font_size = 11.0;
        let text_galley = ui.fonts(|fonts| {
            fonts.layout_no_wrap(
                tag_text.clone(),
                egui::FontId::proportional(font_size),
                text_color,
            )
        });

        // Add padding to the text size
        let padding = egui::vec2(8.0, 4.0);
        let desired_size = text_galley.size() + 2.0 * padding;

        // Allocate space for the tag
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            // Draw rounded rectangle background
            ui.painter().rect_filled(
                rect, 4.0, // corner radius
                tag_color,
            );

            // Draw text centered in the rect
            let text_pos = rect.center() - text_galley.size() / 2.0;
            ui.painter().galley(text_pos, text_galley, text_color);
        }

        // Show tooltip on hover and return click status
        let clicked = response.clicked();
        let tooltip_text = if count == 0 {
            format!(
                "Resource Type: {} - No resources found (click to remove)",
                resource_type
            )
        } else {
            format!(
                "Resource Type: {} - {} resources found (click to remove)",
                resource_type, count
            )
        };
        response.on_hover_text(tooltip_text);
        clicked
    }

    /// Process pending detail requests from the tree renderer and trigger AWS describe calls
    fn process_pending_detail_requests(
        &self,
        state: &ResourceExplorerState,
        ctx: &Context,
        pending_requests: Vec<String>,
    ) {
        for resource_key in pending_requests {
            // Parse the resource key: account_id:region:resource_id
            let parts: Vec<&str> = resource_key.split(':').collect();
            if parts.len() != 3 {
                tracing::warn!("Invalid resource key format: {}", resource_key);
                continue;
            }

            let account_id = parts[0];
            let region = parts[1];
            let resource_id = parts[2];

            // Find the resource in the current state
            if let Some(resource) = state.resources.iter().find(|r| {
                r.account_id == account_id && r.region == region && r.resource_id == resource_id
            }) {
                // Skip if we already have detailed properties
                if resource.detailed_properties.is_some() {
                    continue;
                }

                // Trigger async detailed loading
                self.load_resource_details(resource.clone(), ctx, resource_key.clone());
            }
        }
    }

    /// Load detailed properties for a specific resource using AWS describe APIs
    fn load_resource_details(&self, resource: ResourceEntry, ctx: &Context, resource_key: String) {
        if let Some(ref aws_client) = self.aws_client {
            let client = aws_client.clone();
            let state_arc = Arc::clone(&self.state);
            let ctx_clone = ctx.clone();
            let failed_requests_arc = Arc::clone(&self.failed_detail_requests);

            // Clone the resource for the async task
            let resource_clone = resource.clone();

            // Spawn background thread to avoid blocking UI (following existing pattern)
            std::thread::spawn(move || {
                // Create tokio runtime for async operations
                let runtime = match tokio::runtime::Runtime::new() {
                    Ok(rt) => rt,
                    Err(e) => {
                        tracing::error!(
                            "Failed to create tokio runtime for detailed loading: {}",
                            e
                        );
                        return;
                    }
                };

                // Perform async describe operation
                let result = runtime.block_on(async {
                    tracing::info!(
                        "🔍 Loading detailed properties for: {} ({})",
                        resource_clone.display_name,
                        resource_clone.resource_type
                    );

                    client.describe_resource(&resource_clone).await
                });

                match result {
                    Ok(detailed_properties) => {
                        // Update the resource with detailed properties
                        if let Ok(mut state) = state_arc.try_write() {
                            // Find and update the resource in the state
                            if let Some(existing_resource) = state.resources.iter_mut().find(|r| {
                                r.account_id == resource_clone.account_id
                                    && r.region == resource_clone.region
                                    && r.resource_id == resource_clone.resource_id
                            }) {
                                existing_resource.set_detailed_properties(detailed_properties);
                                tracing::info!(
                                    "✅ Successfully loaded detailed properties for: {}",
                                    existing_resource.display_name
                                );

                                // Request UI repaint to show the updated data
                                ctx_clone.request_repaint();
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "❌ Failed to load detailed properties for {}: {}",
                            resource_clone.display_name,
                            e
                        );

                        // Mark this resource as failed to prevent future retries
                        if let Ok(mut failed_set) = failed_requests_arc.try_write() {
                            failed_set.insert(resource_key);
                            tracing::debug!(
                                "🚫 Marked resource as failed: {}",
                                resource_clone.display_name
                            );
                        }

                        // Request UI repaint to show the failed state
                        ctx_clone.request_repaint();
                    }
                }
            });
        } else {
            tracing::warn!("AWS client not available for loading detailed properties");
        }
    }
}
