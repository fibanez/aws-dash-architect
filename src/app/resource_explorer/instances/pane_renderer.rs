//! Pane Renderer - Handles rendering for a single explorer pane
//!
//! This module extracts pane-specific rendering logic from window.rs,
//! allowing each pane to independently render its tree view, search bar,
//! sidebar, and active selection tags.

use crate::app::resource_explorer::aws_client::AWSResourceClient;
use crate::app::resource_explorer::state::{
    BooleanOperator, GroupingMode, ResourceEntry, ResourceExplorerState, TagClickAction,
    TagFilter, TagFilterGroup, TagFilterType,
};
use crate::app::resource_explorer::tree::TreeRenderer;
use crate::app::resource_explorer::widgets::tag_filter_builder::TagFilterBuilderWidget;
use crate::app::resource_explorer::PropertyFilterGroup;
use egui::{Color32, Context, Ui};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Actions that can be triggered by pane rendering
///
/// These are returned to the window/manager for processing,
/// keeping the pane renderer independent of window-level logic.
#[derive(Debug, Clone)]
pub enum PaneAction {
    /// Remove an account from the query scope
    RemoveAccount(String),
    /// Remove a region from the query scope
    RemoveRegion(String),
    /// Remove a resource type from the query scope
    RemoveResourceType(String),
    /// Open the tag filter builder dialog
    ShowTagFilterBuilder,
    /// Open the property filter builder dialog
    ShowPropertyFilterBuilder,
    /// Open the tag hierarchy builder dialog
    ShowTagHierarchyBuilder,
    /// Open the property hierarchy builder dialog
    ShowPropertyHierarchyBuilder,
}

/// Renderer for a single explorer pane
///
/// Contains rendering state and logic for displaying:
/// - Tree view of resources
/// - Search bar with filtering
/// - Sidebar for grouping mode selection
/// - Active selection tags (accounts, regions, resource types)
pub struct PaneRenderer {
    /// Tree renderer for hierarchical resource display
    pub tree_renderer: TreeRenderer,
    /// Track failed detail requests to avoid retrying
    pub failed_detail_requests: Arc<RwLock<HashSet<String>>>,
    /// Frame counter for debouncing logs and operations
    pub frame_count: u64,
}

impl Default for PaneRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl PaneRenderer {
    /// Create a new pane renderer
    pub fn new() -> Self {
        Self {
            tree_renderer: TreeRenderer::new(),
            failed_detail_requests: Arc::new(RwLock::new(HashSet::new())),
            frame_count: 0,
        }
    }

    /// Increment the frame counter (called each frame)
    pub fn tick(&mut self) {
        self.frame_count = self.frame_count.wrapping_add(1);
    }

    /// Check if a detail request has failed
    pub fn is_failed_request(&self, arn: &str) -> bool {
        self.failed_detail_requests
            .try_read()
            .map(|set| set.contains(arn))
            .unwrap_or(false)
    }

    /// Mark a detail request as failed
    pub async fn mark_request_failed(&self, arn: String) {
        let mut set = self.failed_detail_requests.write().await;
        set.insert(arn);
    }

    /// Clear all failed requests (e.g., on refresh)
    pub async fn clear_failed_requests(&self) {
        let mut set = self.failed_detail_requests.write().await;
        set.clear();
    }

    /// Get the current frame count
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Reset the renderer state (for Terminate action)
    pub fn reset(&mut self) {
        self.tree_renderer = TreeRenderer::new();
        // Clear failed requests synchronously if possible
        if let Ok(mut set) = self.failed_detail_requests.try_write() {
            set.clear();
        }
        self.frame_count = 0;
    }

    // ========================================================================
    // Main Render Method
    // ========================================================================

    /// Render the complete pane content
    ///
    /// This method orchestrates all pane rendering components:
    /// - Active selection tags (accounts, regions, resource types)
    /// - Search bar for filtering
    /// - Tree view of resources
    ///
    /// Returns a list of actions triggered during rendering (e.g., tag removals)
    pub fn render(&mut self, ui: &mut Ui, state: &mut ResourceExplorerState) -> Vec<PaneAction> {
        self.tick();

        let mut actions = Vec::new();

        // Render active selection tags (closeable tags for accounts, regions, resource types)
        if !state.query_scope.is_empty() {
            actions.extend(Self::render_active_tags(ui, state));
            ui.separator();
        }

        // Render search bar
        Self::render_search_bar(ui, state);
        ui.separator();

        // Render tree view (uses self.tree_renderer)
        Self::render_tree_view(ui, state, &mut self.tree_renderer);

        actions
    }

    // ========================================================================
    // Static Rendering Functions (extracted from window.rs)
    // ========================================================================

    /// Render the search bar
    pub fn render_search_bar(ui: &mut Ui, state: &mut ResourceExplorerState) {
        ui.horizontal(|ui| {
            ui.label("Search:");
            ui.text_edit_singleline(&mut state.search_filter);
            if ui.button("Clear").clicked() {
                state.search_filter.clear();
            }
        });
    }

    /// Render the sidebar with grouping controls and filters
    ///
    /// Returns actions for dialogs that need to be shown (handled by window)
    pub fn render_sidebar(ui: &mut Ui, state: &mut ResourceExplorerState) -> Vec<PaneAction> {
        let mut actions = Vec::new();

        ui.vertical(|ui| {
            // Group By section
            ui.label("Group by:");
            ui.add_space(4.0);

            // Primary grouping dropdown with tag-based options
            egui::ComboBox::from_label("")
                .selected_text(state.primary_grouping.display_name())
                .show_ui(ui, |ui| {
                    // Section 1: Built-in groupings
                    ui.label(egui::RichText::new("Built-in").small().weak());
                    for mode in GroupingMode::default_modes() {
                        ui.selectable_value(
                            &mut state.primary_grouping,
                            mode.clone(),
                            mode.display_name(),
                        );
                    }

                    // Separator
                    ui.separator();

                    // Section 2: Tag-based groupings (dynamic)
                    let tag_keys = state.tag_discovery.get_tag_keys_by_popularity();
                    if !tag_keys.is_empty() {
                        ui.label(egui::RichText::new("Tag Groupings").small().weak());

                        for (tag_key, resource_count) in tag_keys.iter().take(20) {
                            // Only show tags with multiple values (can meaningfully group)
                            if let Some(metadata) = state.tag_discovery.get_tag_metadata(tag_key) {
                                if !metadata.has_multiple_values() {
                                    continue; // Skip tags with only 1 value
                                }

                                // Apply minimum resource count filter
                                if *resource_count < state.min_tag_resources_for_grouping {
                                    continue;
                                }

                                let value_count = metadata.value_count();
                                let label = format!(
                                    "Tag: {} ({} resources, {} values)",
                                    tag_key, resource_count, value_count
                                );

                                let mode = GroupingMode::ByTag(tag_key.clone());
                                let response = ui.selectable_value(
                                    &mut state.primary_grouping,
                                    mode,
                                    label,
                                );

                                // Add tooltip with value distribution preview
                                if response.hovered() {
                                    let values = metadata.get_sorted_values();
                                    let preview = values
                                        .iter()
                                        .take(5)
                                        .map(|v| v.as_str())
                                        .collect::<Vec<_>>()
                                        .join(", ");
                                    let more = if values.len() > 5 {
                                        format!(" ...and {} more", values.len() - 5)
                                    } else {
                                        String::new()
                                    };
                                    response.on_hover_text(format!("Values: {}{}", preview, more));
                                }
                            }
                        }

                        ui.separator();
                    }

                    // Section 3: Tag Hierarchy option
                    ui.label(egui::RichText::new("Advanced").small().weak());
                    if ui.button("Tag Hierarchy...").clicked() {
                        tracing::info!("Tag Hierarchy builder clicked");
                        actions.push(PaneAction::ShowTagHierarchyBuilder);
                    }
                    if ui.button("Property Hierarchy...").clicked() {
                        tracing::info!("Property Hierarchy builder clicked");
                        actions.push(PaneAction::ShowPropertyHierarchyBuilder);
                    }
                });

            ui.add_space(8.0);

            // Min Resources control (below Group By dropdown)
            ui.label("Min res:");
            let drag_response = ui.add(
                egui::DragValue::new(&mut state.min_tag_resources_for_grouping)
                    .speed(1.0)
                    .range(1..=100),
            );
            drag_response.on_hover_text(
                "Minimum number of resources for tags to appear in GroupBy dropdown. Drag to adjust or click to type.",
            );

            ui.separator();
            ui.add_space(8.0);

            // Tag presence checkboxes
            let mut show_tagged = state.show_only_tagged;
            if ui
                .checkbox(&mut show_tagged, "Show only tagged")
                .on_hover_text("Show only resources with any tags")
                .changed()
            {
                state.show_only_tagged = show_tagged;
                // Ensure mutual exclusivity
                if show_tagged {
                    state.show_only_untagged = false;
                }
                tracing::info!(
                    "Tag filter changed: show_only_tagged={}",
                    state.show_only_tagged
                );
            }

            let mut show_untagged = state.show_only_untagged;
            if ui
                .checkbox(&mut show_untagged, "Show only untagged")
                .on_hover_text("Show only resources with no tags")
                .changed()
            {
                state.show_only_untagged = show_untagged;
                // Ensure mutual exclusivity
                if show_untagged {
                    state.show_only_tagged = false;
                }
                tracing::info!(
                    "Tag filter changed: show_only_untagged={}",
                    state.show_only_untagged
                );
            }

            ui.add_space(8.0);

            // Filter buttons stacked vertically
            let advanced_count = state.tag_filter_group.filter_count();
            let property_filter_count = state.property_filter_group.total_filter_count();
            let presence_count = state.tag_presence_filter_count();
            let total_filter_count = presence_count + advanced_count + property_filter_count;

            // Tag Filters button
            if ui
                .button("Tag Filters...")
                .on_hover_text("Open advanced tag filter builder")
                .clicked()
            {
                actions.push(PaneAction::ShowTagFilterBuilder);
            }
            if advanced_count > 0 {
                let filter_text = if advanced_count == 1 {
                    "(1 filter active)".to_string()
                } else {
                    format!("({} filters active)", advanced_count)
                };
                // Bright red text on light yellow background for visibility
                let label = egui::Label::new(
                    egui::RichText::new(filter_text)
                        .color(egui::Color32::from_rgb(200, 40, 40))
                        .strong(),
                );
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(255, 255, 200))
                    .inner_margin(egui::Margin::symmetric(6, 2))
                    .corner_radius(3.0)
                    .show(ui, |ui| {
                        ui.add(label);
                    });
            }

            ui.add_space(4.0);

            // Property Filters button
            if ui
                .button("Property Filters...")
                .on_hover_text("Open property filter builder")
                .clicked()
            {
                actions.push(PaneAction::ShowPropertyFilterBuilder);
            }
            if property_filter_count > 0 {
                let filter_text = if property_filter_count == 1 {
                    "(1 filter active)".to_string()
                } else {
                    format!("({} filters active)", property_filter_count)
                };
                // Bright red text on light yellow background for visibility
                let label = egui::Label::new(
                    egui::RichText::new(filter_text)
                        .color(egui::Color32::from_rgb(200, 40, 40))
                        .strong(),
                );
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(255, 255, 200))
                    .inner_margin(egui::Margin::symmetric(6, 2))
                    .corner_radius(3.0)
                    .show(ui, |ui| {
                        ui.add(label);
                    });
            }

            ui.add_space(4.0);

            // Clear Filters button (only show if filters are active)
            if total_filter_count > 0
                && ui
                    .button("Clear Filters")
                    .on_hover_text("Clear all tag and property filters")
                    .clicked()
            {
                // Clear all tag filters
                state.show_only_tagged = false;
                state.show_only_untagged = false;
                state.tag_filter_group = TagFilterGroup::new();

                // Clear all property filters
                state.property_filter_group = PropertyFilterGroup::new();

                tracing::info!("Cleared all filters (tags and properties)");
            }
        });

        actions
    }

    /// Render the tree view of resources
    pub fn render_tree_view(
        ui: &mut Ui,
        state: &ResourceExplorerState,
        tree_renderer: &mut TreeRenderer,
    ) {
        // Update Phase 2 status for tree renderer
        tree_renderer.phase2_in_progress = state.phase2_enrichment_in_progress;

        // Use remaining available space for the tree view with scrolling
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
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
                    // Apply all filters (tag + property) before rendering
                    let filtered_resources: Vec<_> = state
                        .resources
                        .iter()
                        .filter(|resource| {
                            Self::apply_tag_filters(resource, state)
                                && Self::apply_property_filters(resource, state)
                        })
                        .cloned()
                        .collect();

                    // Show filter stats if filters are active
                    let tag_filter_count =
                        state.tag_presence_filter_count() + state.tag_filter_group.filter_count();
                    let property_filter_count = state.property_filter_group.total_filter_count();
                    let total_filter_count = tag_filter_count + property_filter_count;
                    if total_filter_count > 0 {
                        ui.horizontal(|ui| {
                            ui.label(format!(
                                "Showing {} of {} resources ({} filter{})",
                                filtered_resources.len(),
                                state.resources.len(),
                                total_filter_count,
                                if total_filter_count == 1 { "" } else { "s" }
                            ));
                        });
                        ui.separator();
                    }

                    if filtered_resources.is_empty() {
                        ui.centered_and_justified(|ui| {
                            ui.label("No resources match the active tag filters");
                        });
                    } else {
                        // Use cached tree rendering to prevent unnecessary rebuilds
                        tree_renderer.render_tree_cached(
                            ui,
                            &filtered_resources,
                            state.primary_grouping.clone(),
                            &state.search_filter,
                            &state.badge_selector,
                            &state.tag_popularity,
                            state.enrichment_version,
                        );
                    }
                } else if state.is_loading() {
                    ui.centered_and_justified(|ui| {
                        ui.spinner();
                        ui.label("Loading resources...");
                    });
                }
            });
    }

    /// Render active selection tags and return any removal actions
    pub fn render_active_tags(
        ui: &mut Ui,
        state: &mut ResourceExplorerState,
    ) -> Vec<PaneAction> {
        let mut actions = Vec::new();

        // Count total tags
        let total_accounts = state.query_scope.accounts.len();
        let total_regions = state.query_scope.regions.len();
        let total_resource_types = state.query_scope.resource_types.len();
        let total_tags = total_accounts + total_regions + total_resource_types;

        // Number of tags to show when collapsed
        const COLLAPSED_TAG_LIMIT: usize = 5;
        let hidden_count = total_tags.saturating_sub(COLLAPSED_TAG_LIMIT);

        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Selection:").small());

            // Show expand/collapse button if there are hidden tags
            if hidden_count > 0 {
                let button_text = if state.active_selection_expanded {
                    "[-]".to_string()
                } else {
                    format!("[+{}]", hidden_count)
                };
                if ui.small_button(&button_text).clicked() {
                    state.active_selection_expanded = !state.active_selection_expanded;
                }
            }
        });

        ui.horizontal_wrapped(|ui| {
            ui.set_max_width(ui.available_width());
            let mut tags_shown = 0;
            let show_all = state.active_selection_expanded || hidden_count == 0;

            // Account tags with colored tag rendering
            for account in &state.query_scope.accounts {
                if !show_all && tags_shown >= COLLAPSED_TAG_LIMIT {
                    break;
                }
                if Self::render_closeable_account_tag(
                    ui,
                    &account.account_id,
                    &account.display_name,
                    account.color,
                ) {
                    actions.push(PaneAction::RemoveAccount(account.account_id.clone()));
                }
                ui.add_space(2.0);
                tags_shown += 1;
            }

            // Region tags with colored tag rendering
            for region in &state.query_scope.regions {
                if !show_all && tags_shown >= COLLAPSED_TAG_LIMIT {
                    break;
                }
                if Self::render_closeable_region_tag(
                    ui,
                    &region.region_code,
                    &region.display_name,
                    region.color,
                ) {
                    actions.push(PaneAction::RemoveRegion(region.region_code.clone()));
                }
                ui.add_space(2.0);
                tags_shown += 1;
            }

            // Resource type tags with count
            for resource_type in &state.query_scope.resource_types {
                if !show_all && tags_shown >= COLLAPSED_TAG_LIMIT {
                    break;
                }
                // Count resources for this resource type
                let resource_count = state
                    .resources
                    .iter()
                    .filter(|r| r.resource_type == resource_type.resource_type)
                    .count();

                if Self::render_closeable_resource_type_tag_with_count(
                    ui,
                    &resource_type.resource_type,
                    &resource_type.display_name,
                    resource_count,
                ) {
                    actions.push(PaneAction::RemoveResourceType(
                        resource_type.resource_type.clone(),
                    ));
                }
                ui.add_space(2.0);
                tags_shown += 1;
            }
        });

        actions
    }

    // ========================================================================
    // Static Filter Functions (extracted from window.rs)
    // ========================================================================

    /// Apply all tag filters to a resource (presence/absence + advanced filters)
    pub fn apply_tag_filters(resource: &ResourceEntry, state: &ResourceExplorerState) -> bool {
        // First, apply presence/absence filters
        let presence_filter_active = state.show_only_tagged || state.show_only_untagged;

        if presence_filter_active {
            let has_tags = !resource.tags.is_empty();

            // Show only tagged: pass resources with tags
            if state.show_only_tagged && !has_tags {
                return false;
            }

            // Show only untagged: pass resources without tags
            if state.show_only_untagged && has_tags {
                return false;
            }
        }

        // Then, apply advanced filter group
        // Empty filter groups match everything (no filtering)
        if !state.tag_filter_group.is_empty() && !state.tag_filter_group.matches(resource) {
            return false;
        }

        true
    }

    /// Apply property filters to a resource
    pub fn apply_property_filters(resource: &ResourceEntry, state: &ResourceExplorerState) -> bool {
        // Empty filter groups match everything (no filtering)
        if state.property_filter_group.is_empty() {
            return true;
        }

        // Apply the property filter group
        state
            .property_filter_group
            .matches(&resource.resource_id, &state.property_catalog)
    }

    /// Process tag badge clicks by adding filters to the filter group
    pub fn process_tag_badge_clicks(
        state: &mut ResourceExplorerState,
        clicks: Vec<TagClickAction>,
    ) {
        for click in clicks {
            // Create the filter for this tag
            let new_filter = TagFilter {
                tag_key: click.tag_key.clone(),
                filter_type: TagFilterType::Equals,
                values: vec![click.tag_value.clone()],
                pattern: None,
            };

            // Check if existing filter group is empty
            if state.tag_filter_group.is_empty() {
                // No existing filters - add as first filter
                state.tag_filter_group.add_filter(new_filter);

                tracing::info!(
                    "Added first filter: {} = {}",
                    click.tag_key,
                    click.tag_value
                );
            } else {
                // Existing filters - add as new sub-group with OR operator
                let mut new_sub_group = TagFilterGroup::new();
                new_sub_group.operator = BooleanOperator::Or;
                new_sub_group.add_filter(new_filter);

                state.tag_filter_group.add_sub_group(new_sub_group);

                tracing::info!(
                    "Added filter as sub-group: {} = {} (combined with OR)",
                    click.tag_key,
                    click.tag_value
                );
            }

            // Log the resulting expression for visibility
            let filter_expr =
                TagFilterBuilderWidget::format_filter_expression(&state.tag_filter_group, 0);
            tracing::info!("Updated filter expression: {}", filter_expr);
        }

        // Filters have changed, which will trigger tree rebuild on next frame
    }

    /// Process pending detail requests and trigger AWS describe calls
    ///
    /// This is a static function that can be called with any pane's state and dependencies.
    pub fn process_pending_detail_requests(
        state: &ResourceExplorerState,
        ctx: &Context,
        pending_requests: Vec<String>,
        aws_client: &Arc<AWSResourceClient>,
        state_arc: &Arc<RwLock<ResourceExplorerState>>,
        failed_requests_arc: &Arc<RwLock<HashSet<String>>>,
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
                Self::load_resource_details(
                    resource.clone(),
                    ctx,
                    resource_key.clone(),
                    aws_client,
                    state_arc,
                    failed_requests_arc,
                );
            }
        }
    }

    /// Load detailed properties for a specific resource using AWS describe APIs
    ///
    /// This is a static function that can be called with any pane's dependencies.
    pub fn load_resource_details(
        resource: ResourceEntry,
        ctx: &Context,
        resource_key: String,
        aws_client: &Arc<AWSResourceClient>,
        state_arc: &Arc<RwLock<ResourceExplorerState>>,
        failed_requests_arc: &Arc<RwLock<HashSet<String>>>,
    ) {
        let client = aws_client.clone();
        let state_arc = Arc::clone(state_arc);
        let ctx_clone = ctx.clone();
        let failed_requests_arc = Arc::clone(failed_requests_arc);

        // Clone the resource for the async task
        let resource_clone = resource.clone();

        // Spawn background thread to avoid blocking UI
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
                    "Loading detailed properties for: {} ({})",
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
                                "Successfully loaded detailed properties for: {}",
                                existing_resource.display_name
                            );

                            // Request UI repaint to show the updated data
                            ctx_clone.request_repaint();
                        }
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to load detailed properties for {}: {}",
                        resource_clone.display_name,
                        e
                    );

                    // Mark this resource as failed to prevent future retries
                    if let Ok(mut failed_set) = failed_requests_arc.try_write() {
                        failed_set.insert(resource_key);
                        tracing::debug!(
                            "Marked resource as failed: {}",
                            resource_clone.display_name
                        );
                    }

                    // Request UI repaint to show the failed state
                    ctx_clone.request_repaint();
                }
            }
        });
    }

    // ========================================================================
    // Static Tag Rendering Helpers (extracted from window.rs)
    // ========================================================================

    /// Render a closeable account tag with color
    /// Returns true if the close button was clicked
    fn render_closeable_account_tag(
        ui: &mut Ui,
        account_id: &str,
        display_name: &str,
        color: Color32,
    ) -> bool {
        let text_color = crate::app::resource_explorer::colors::get_contrasting_text_color(color);
        let label_text = if display_name.is_empty() || display_name == account_id {
            format!("Account: {}", account_id)
        } else {
            format!("Account: {} ({})", display_name, account_id)
        };

        let mut clicked = false;
        egui::Frame::new()
            .fill(color)
            .corner_radius(3.0)
            .inner_margin(egui::Margin::symmetric(5, 2))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    ui.label(egui::RichText::new(&label_text).size(9.0).color(text_color));
                    // Clickable x label instead of button - matches tag background
                    let x_response = ui.add(
                        egui::Label::new(
                            egui::RichText::new("x")
                                .size(9.0)
                                .color(text_color)
                        ).sense(egui::Sense::click())
                    );
                    if x_response.clicked() {
                        clicked = true;
                    }
                });
            });
        clicked
    }

    /// Render a closeable region tag with color
    /// Returns true if the close button was clicked
    fn render_closeable_region_tag(
        ui: &mut Ui,
        region_code: &str,
        display_name: &str,
        color: Color32,
    ) -> bool {
        let text_color = crate::app::resource_explorer::colors::get_contrasting_text_color(color);
        let label_text = if display_name.is_empty() || display_name == region_code {
            format!("Region: {}", region_code)
        } else {
            format!("Region: {} ({})", display_name, region_code)
        };

        let mut clicked = false;
        egui::Frame::new()
            .fill(color)
            .corner_radius(3.0)
            .inner_margin(egui::Margin::symmetric(5, 2))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    ui.label(egui::RichText::new(&label_text).size(9.0).color(text_color));
                    // Clickable x label instead of button - matches tag background
                    let x_response = ui.add(
                        egui::Label::new(
                            egui::RichText::new("x")
                                .size(9.0)
                                .color(text_color)
                        ).sense(egui::Sense::click())
                    );
                    if x_response.clicked() {
                        clicked = true;
                    }
                });
            });
        clicked
    }

    /// Render a closeable resource type tag with count
    /// Returns true if the close button was clicked
    fn render_closeable_resource_type_tag_with_count(
        ui: &mut Ui,
        resource_type: &str,
        display_name: &str,
        count: usize,
    ) -> bool {
        let label_text = if display_name.is_empty() || display_name == resource_type {
            format!("{} ({})", resource_type, count)
        } else {
            format!("{} ({})", display_name, count)
        };

        let mut clicked = false;
        egui::Frame::new()
            .fill(Color32::from_rgb(100, 100, 100))
            .corner_radius(3.0)
            .inner_margin(egui::Margin::symmetric(5, 2))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    ui.label(egui::RichText::new(&label_text).size(9.0).color(Color32::WHITE));
                    // Clickable x label instead of button - matches tag background
                    let x_response = ui.add(
                        egui::Label::new(
                            egui::RichText::new("x")
                                .size(9.0)
                                .color(Color32::WHITE)
                        ).sense(egui::Sense::click())
                    );
                    if x_response.clicked() {
                        clicked = true;
                    }
                });
            });
        clicked
    }
}
