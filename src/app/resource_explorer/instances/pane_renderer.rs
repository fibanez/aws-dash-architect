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
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Global cache for memory stats to avoid reading /proc every frame
static MEMORY_STATS_CACHE: Mutex<Option<(memory_stats::MemoryStats, Instant)>> =
    Mutex::new(None);
const MEMORY_STATS_CACHE_DURATION: Duration = Duration::from_millis(500);

/// Get cached memory stats, refreshing at most every 500ms
///
/// This avoids reading /proc/self/status on every frame for every pane,
/// which was causing 2 MB * 2,350 reads = 4.7 GB of allocations in 2.2 seconds.
fn get_cached_memory_stats() -> Option<memory_stats::MemoryStats> {
    let mut cache = MEMORY_STATS_CACHE.lock().ok()?;

    // Check if we have a recent cached value
    if let Some((stats, timestamp)) = cache.as_ref() {
        if timestamp.elapsed() < MEMORY_STATS_CACHE_DURATION {
            return Some(stats.clone());
        }
    }

    // Cache expired or doesn't exist - fetch new stats
    if let Some(new_stats) = memory_stats::memory_stats() {
        *cache = Some((new_stats.clone(), Instant::now()));
        Some(new_stats)
    } else {
        None
    }
}

/// Actions that can be triggered by pane rendering
///
/// These are returned to the window/manager for processing,
/// keeping the pane renderer independent of window-level logic.
#[derive(Debug, Clone)]
pub enum PaneAction {
    /// Remove an account from the query scope
    /// Contains account_id and the source pane_id that requested it
    RemoveAccount { account_id: String, source_pane_id: Uuid },
    /// Remove a region from the query scope
    /// Contains region_code and the source pane_id that requested it
    RemoveRegion { region_code: String, source_pane_id: Uuid },
    /// Remove a resource type from the query scope
    /// Contains resource_type and the source pane_id that requested it
    RemoveResourceType { resource_type: String, source_pane_id: Uuid },
    /// Open the tag filter builder dialog
    ShowTagFilterBuilder,
    /// Open the property filter builder dialog
    ShowPropertyFilterBuilder,
    /// Open the tag hierarchy builder dialog
    ShowTagHierarchyBuilder,
    /// Open the property hierarchy builder dialog
    ShowPropertyHierarchyBuilder,
    /// Apply a bookmark to this pane's state
    /// Contains bookmark_id and the source pane_id that requested it
    ApplyBookmark { bookmark_id: String, source_pane_id: Uuid },
    /// Show the failed queries dialog with error details
    ShowFailedQueriesDialog,
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

    /// Take pending ResourceExplorerActions from the tree renderer
    ///
    /// These are actions like opening CloudWatch Logs, CloudTrail Events, or AWS Console
    /// that should be processed by the main application.
    pub fn take_pending_actions(&mut self) -> Vec<crate::app::resource_explorer::ResourceExplorerAction> {
        std::mem::take(&mut self.tree_renderer.pending_explorer_actions)
    }

    // ========================================================================
    // Main Render Method
    // ========================================================================

    /// Render the complete pane content with unique IDs
    ///
    /// This method orchestrates all pane rendering components:
    /// - Left sidebar with grouping controls and filters
    /// - Active selection tags (accounts, regions, resource types)
    /// - Search bar for filtering
    /// - Tree view of resources
    ///
    /// The pane_id parameter is used to make all widget IDs unique across panes.
    ///
    /// Returns a list of actions triggered during rendering (e.g., tag removals)
    pub fn render_with_id(
        &mut self,
        ui: &mut Ui,
        state: &mut ResourceExplorerState,
        pane_id: Uuid,
        shared_context: &super::manager::ExplorerSharedContext,
    ) -> Vec<PaneAction> {
        self.tick();

        let mut actions = Vec::new();

        // Left sidebar for grouping and filter controls
        // Use pane ID to make sidebar unique across split panes
        egui::SidePanel::left(format!("explorer_sidebar_{}", pane_id))
            .default_width(180.0)
            .min_width(150.0)
            .resizable(true)
            .show_inside(ui, |ui| {
                actions.extend(Self::render_sidebar(ui, state));
            });

        // Central panel with main content
        egui::CentralPanel::default().show_inside(ui, |ui| {
            // Bottom status bar (at pane level, not window level)
            egui::TopBottomPanel::bottom(format!("pane_status_bar_{}", pane_id))
                .show_separator_line(false) // No thick horizontal line at bottom
                .show_inside(ui, |ui| {
                    if let Some(status_action) = Self::render_status_bar(ui, state) {
                        actions.push(status_action);
                    }
                });

            // Main content area
            egui::CentralPanel::default().show_inside(ui, |ui| {
                // Render toolbar (Bookmarks, Select, Refresh, Reset, Cache)
                if let Some((bookmark_id, source_pane_id)) = Self::render_toolbar(ui, state, shared_context, pane_id) {
                    actions.push(PaneAction::ApplyBookmark { bookmark_id, source_pane_id });
                }
                ui.separator();

                // Render active selection tags (closeable tags for accounts, regions, resource types)
                if !state.query_scope.is_empty() {
                    actions.extend(Self::render_active_tags(ui, state, pane_id));
                    ui.separator();
                }

                // Render search bar
                Self::render_search_bar(ui, state);
                ui.separator();

                // Render tree view with unique ID (uses self.tree_renderer)
                Self::render_tree_view_with_id(ui, state, &mut self.tree_renderer, pane_id);
            });
        });

        actions
    }

    /// Legacy render method (kept for backwards compatibility)
    ///
    /// Calls render_with_id with a default UUID and empty shared context
    #[allow(dead_code)]
    pub fn render(
        &mut self,
        ui: &mut Ui,
        state: &mut ResourceExplorerState,
        shared_context: &super::manager::ExplorerSharedContext,
    ) -> Vec<PaneAction> {
        self.render_with_id(ui, state, Uuid::new_v4(), shared_context)
    }

    // ========================================================================
    // Static Rendering Functions (extracted from window.rs)
    // ========================================================================

    /// Render the toolbar with main action buttons
    ///
    /// Based on window.rs render_unified_toolbar (lines 1808-1942)
    /// Returns: clicked_bookmark_id
    pub fn render_toolbar(
        ui: &mut Ui,
        state: &mut ResourceExplorerState,
        shared_context: &super::manager::ExplorerSharedContext,
        pane_id: Uuid,
    ) -> Option<(String, Uuid)> {
        let mut clicked_bookmark_id: Option<String> = None;

        ui.horizontal(|ui| {
            // Bookmarks menu button with full hierarchy
            ui.menu_button("Bookmarks", |ui| {
                // Render top-level bookmarks and folders
                Self::render_bookmark_menu_level(
                    ui,
                    None, // Top level (no parent folder)
                    state,
                    shared_context,
                    &mut clicked_bookmark_id,
                );

                ui.separator();
                if ui.button("Add Bookmark").clicked() {
                    state.show_bookmark_dialog = true;
                    ui.close();
                }
                if ui.button("Manage Bookmarks").clicked() {
                    state.show_bookmark_manager = true;
                    ui.close();
                }
            });

            // Separator before action buttons
            if ui.available_width() > 400.0 {
                ui.separator();
            }

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
                state.clear_all_selections();
            }

            // TODO: Add Verify with CLI button (DEBUG only)

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

        clicked_bookmark_id.map(|id| (id, pane_id))
    }

    /// Recursively render a level of the bookmark menu hierarchy
    ///
    /// Based on window.rs render_bookmark_menu_level (lines 1948-2037)
    fn render_bookmark_menu_level(
        ui: &mut Ui,
        parent_folder_id: Option<String>,
        state: &ResourceExplorerState,
        shared_context: &super::manager::ExplorerSharedContext,
        clicked_bookmark_id: &mut Option<String>,
    ) {
        // Get bookmarks at this level
        let bookmarks: Vec<_> = shared_context
            .bookmarks
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
        let folders: Vec<_> = shared_context
            .bookmarks
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
                Self::render_bookmark_menu_level(
                    ui,
                    Some(folder.id.clone()),
                    state,
                    shared_context,
                    clicked_bookmark_id,
                );
            });
        }

        // Show "empty" message if no bookmarks or folders at this level
        if bookmarks.is_empty() && folders.is_empty() {
            ui.label(egui::RichText::new("(no bookmarks)").italics().weak());
        }
    }

    /// Render the status bar at the bottom of the pane
    ///
    /// Shows loading progress, failed queries, memory stats, and cache info
    /// Based on window.rs lines 448-837
    /// Returns: action to show failed queries dialog if clicked
    pub fn render_status_bar(ui: &mut Ui, state: &ResourceExplorerState) -> Option<PaneAction> {
        let mut action = None;
        ui.horizontal(|ui| {
            // Left section (scrollable for long status messages)
            egui::ScrollArea::horizontal()
                .id_salt("status_bar_scroll")
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .show(ui, |ui| {
                    // Check for Phase 1 (resource listing) progress
                    if state.is_phase1_in_progress() {
                        let (pending_count, total, failed_count, _pending_list) = state.get_phase1_progress();
                        ui.spinner();

                        let status_text = if failed_count > 0 {
                            format!(
                                "Phase 1: Loading resources... ({}/{}, {} left, {} failed)",
                                total - pending_count,
                                total,
                                pending_count,
                                failed_count
                            )
                        } else if pending_count > 0 {
                            format!(
                                "Phase 1: Loading resources... ({}/{}, {} left)",
                                total - pending_count,
                                total,
                                pending_count
                            )
                        } else {
                            format!(
                                "Phase 1: Loading resources... ({}/{})",
                                total - pending_count,
                                total
                            )
                        };

                        ui.label(
                            egui::RichText::new(status_text)
                                .color(Color32::from_rgb(100, 180, 255))
                                .small(),
                        );
                    } else if state.phase2_enrichment_in_progress {
                        // Phase 2 enrichment progress with service and count
                        ui.spinner();
                        let progress_text = if let Some(ref service) = state.phase2_current_service {
                            format!(
                                "Phase 2: {} ({}/{})",
                                service,
                                state.phase2_progress_count,
                                state.phase2_progress_total
                            )
                        } else {
                            format!(
                                "Phase 2: Enriching details... ({}/{})",
                                state.phase2_progress_count,
                                state.phase2_progress_total
                            )
                        };
                        ui.label(
                            egui::RichText::new(progress_text)
                                .color(Color32::from_rgb(255, 180, 100))
                                .small(),
                        );
                    } else if state.is_loading() {
                        // Generic loading state
                        ui.spinner();
                        ui.label(
                            egui::RichText::new("Loading...")
                                .color(Color32::from_rgb(100, 180, 255))
                                .small(),
                        );
                    } else {
                        // Ready state
                        ui.label(
                            egui::RichText::new("Ready")
                                .color(Color32::GRAY)
                                .small(),
                        );
                    }

                    // Show failed queries indicator (persistent after Phase 1 completes) - CLICKABLE
                    let failed_count = state.phase1_failed_queries.len();
                    if failed_count > 0 {
                        let response = ui.add(
                            egui::Label::new(
                                egui::RichText::new(format!("[{} queries failed]", failed_count))
                                    .color(Color32::from_rgb(255, 150, 50))
                                    .small()
                            ).sense(egui::Sense::click())
                        );

                        if response.clicked() {
                            action = Some(PaneAction::ShowFailedQueriesDialog);
                        }

                        response.on_hover_ui(|ui| {
                            ui.label(egui::RichText::new("Failed Queries").strong());
                            ui.separator();
                            ui.label("Some queries failed due to errors or regional unavailability.");
                            ui.label("This may be due to permissions, service availability, or network issues.");
                            ui.add_space(4.0);
                            ui.label(egui::RichText::new("Click to see details and error categories.").weak());
                        });
                    }
                });

            // Right section: Memory and cache stats
            // Format: "220MB | 45.2MB cache | 150 active, 234 queries"
            // - 220MB: Physical memory used by application process
            // - 45.2MB cache: Compressed cache size
            // - 150 active: Number of resources currently displayed in this pane
            // - 234 queries: Number of resource queries cached
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let shared_cache = super::super::cache::shared_cache();
                shared_cache.run_pending_tasks();
                let cache_stats = shared_cache.memory_stats();
                let active_count = state.resources.len();

                if let Some(usage) = get_cached_memory_stats() {
                    let physical_mb = usage.physical_mem as f64 / (1024.0 * 1024.0);
                    let cache_mb = cache_stats.total_size() as f64 / (1024.0 * 1024.0);

                    let cache_info = if cache_stats.resource_entry_count > 0 {
                        format!(
                            "{:.1}MB cache | {} active, {} queries",
                            cache_mb, active_count, cache_stats.resource_entry_count
                        )
                    } else {
                        format!("{} active", active_count)
                    };

                    ui.label(
                        egui::RichText::new(format!("{:.0}MB | {}", physical_mb, cache_info))
                            .small()
                            .color(Color32::GRAY),
                    );
                } else {
                    let cache_mb = cache_stats.total_size() as f64 / (1024.0 * 1024.0);
                    ui.label(
                        egui::RichText::new(format!("{} active, {:.1}MB cached", active_count, cache_mb))
                            .small()
                            .color(Color32::GRAY),
                    );
                }
            });
        });
        action
    }

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

    /// Render the tree view of resources with unique ID
    pub fn render_tree_view_with_id(
        ui: &mut Ui,
        state: &ResourceExplorerState,
        tree_renderer: &mut TreeRenderer,
        pane_id: Uuid,
    ) {
        // Update Phase 2 status for tree renderer
        tree_renderer.phase2_in_progress = state.phase2_enrichment_in_progress;

        // Use remaining available space for the tree view with scrolling
        // Use pane_id to make ScrollArea unique across split panes
        egui::ScrollArea::vertical()
            .id_salt(format!("tree_scroll_{}", pane_id))
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

    /// Legacy render_tree_view (for backwards compatibility)
    #[allow(dead_code)]
    pub fn render_tree_view(
        ui: &mut Ui,
        state: &ResourceExplorerState,
        tree_renderer: &mut TreeRenderer,
    ) {
        Self::render_tree_view_with_id(ui, state, tree_renderer, Uuid::new_v4())
    }

    /// Render active selection tags and return any removal actions
    pub fn render_active_tags(
        ui: &mut Ui,
        state: &mut ResourceExplorerState,
        pane_id: Uuid,
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
                    actions.push(PaneAction::RemoveAccount {
                        account_id: account.account_id.clone(),
                        source_pane_id: pane_id,
                    });
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
                    actions.push(PaneAction::RemoveRegion {
                        region_code: region.region_code.clone(),
                        source_pane_id: pane_id,
                    });
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
                    actions.push(PaneAction::RemoveResourceType {
                        resource_type: resource_type.resource_type.clone(),
                        source_pane_id: pane_id,
                    });
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
                if resource.detailed_timestamp.is_some() {
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
                Ok(_detailed_properties) => {
                    // Update the resource with detailed properties
                    if let Ok(mut state) = state_arc.try_write() {
                        // Find and update the resource in the state
                        if let Some(existing_resource) = state.resources.iter_mut().find(|r| {
                            r.account_id == resource_clone.account_id
                                && r.region == resource_clone.region
                                && r.resource_id == resource_clone.resource_id
                        }) {
                            existing_resource.mark_enriched(); // detailed_properties);
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
        _color: Color32, // Ignored - using consistent yellow per UI spec
    ) -> bool {
        // Per UI spec (EXPLORER_UI_MAP.md line 293): All accounts use yellow background
        let color = Color32::from_rgb(255, 220, 100);
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
        _color: Color32, // Ignored - using consistent light green per UI spec
    ) -> bool {
        // Per UI spec (EXPLORER_UI_MAP.md line 297): All regions use light green background
        let color = Color32::from_rgb(144, 238, 144);
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

        // Resource types use dynamic grey that works in all themes
        let color = if ui.visuals().dark_mode {
            Color32::from_rgb(80, 80, 80) // Dark grey for dark theme
        } else {
            Color32::from_rgb(200, 200, 200) // Light grey for light theme
        };
        let text_color = crate::app::resource_explorer::colors::get_contrasting_text_color(color);

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
}
