use super::{colors::*, state::*};
use egui::{Color32, RichText, Ui};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::collections::HashMap;

/// Tree node structure for hierarchical display
#[derive(Debug, Clone)]
pub struct TreeNode {
    pub id: String,
    pub display_name: String,
    pub color: Option<Color32>,
    pub children: Vec<TreeNode>,
    pub resource_entries: Vec<ResourceEntry>, // Leaf nodes contain actual resources
    pub expanded: bool,
    pub node_type: NodeType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeType {
    Account,
    Region,
    ResourceType,
    Resource,
}

impl TreeNode {
    pub fn new(id: String, display_name: String, node_type: NodeType) -> Self {
        Self {
            id,
            display_name,
            color: None,
            children: Vec::new(),
            resource_entries: Vec::new(),
            expanded: false,
            node_type,
        }
    }

    /// Create a stable node ID based on name:account:region:resource_type for uniqueness
    /// This ensures CollapsingHeader state persists across tree rebuilds and prevents ID conflicts
    pub fn create_stable_node_id(
        account_id: &str,
        region: &str,
        resource_type: &str,
        resource_name: Option<&str>,
        resource_id: Option<&str>,
    ) -> String {
        match (resource_name, resource_id) {
            (Some(name), Some(id)) => format!(
                "{}:{}:{}:{}:{}",
                name, account_id, region, resource_type, id
            ),
            (Some(name), None) => format!("{}:{}:{}:{}", name, account_id, region, resource_type),
            (None, Some(id)) => format!("{}:{}:{}:{}", account_id, region, resource_type, id),
            (None, None) => format!("{}:{}:{}", account_id, region, resource_type),
        }
    }

    /// Create a stable group node ID for account/region/resource_type combinations
    pub fn create_group_node_id(
        grouping_mode: &crate::app::resource_explorer::state::GroupingMode,
        primary_key: &str,
        secondary_key: Option<&str>,
    ) -> String {
        match secondary_key {
            Some(secondary) => format!("{:?}:{}:{}", grouping_mode, primary_key, secondary),
            None => format!("{:?}:{}", grouping_mode, primary_key),
        }
    }

    pub fn with_color(mut self, color: Color32) -> Self {
        self.color = Some(color);
        self
    }

    pub fn add_child(&mut self, child: TreeNode) {
        self.children.push(child);
    }

    pub fn add_resource(&mut self, resource: ResourceEntry) {
        self.resource_entries.push(resource);
    }

    pub fn is_leaf(&self) -> bool {
        self.children.is_empty()
    }

    pub fn total_resources(&self) -> usize {
        self.resource_entries.len()
            + self
                .children
                .iter()
                .map(|c| c.total_resources())
                .sum::<usize>()
    }
}

/// Tree builder for creating hierarchical structure from flat resource list
pub struct TreeBuilder;

impl TreeBuilder {
    pub fn build_tree(
        resources: &[ResourceEntry],
        primary_grouping: GroupingMode,
        search_filter: &str,
    ) -> TreeNode {
        // Only start search filtering after 3 characters to reduce tree rebuilds
        let filtered_resources = if search_filter.len() < 3 {
            resources.to_vec()
        } else {
            Self::filter_resources(resources, search_filter)
        };

        let mut root = TreeNode::new(
            "root".to_string(),
            "AWS Resources".to_string(),
            NodeType::Resource,
        );

        // Group by primary grouping
        let primary_groups = Self::group_by_mode(&filtered_resources, &primary_grouping);

        for (primary_key, primary_resources) in primary_groups {
            let (primary_display, primary_color) =
                Self::get_display_info(&primary_key, &primary_grouping, &primary_resources);

            // Create stable node ID for primary grouping
            let primary_node_id =
                TreeNode::create_group_node_id(&primary_grouping, &primary_key, None);
            let mut primary_node = TreeNode::new(
                primary_node_id,
                primary_display,
                Self::grouping_to_node_type(&primary_grouping),
            );

            if let Some(color) = primary_color {
                primary_node = primary_node.with_color(color);
            }

            // Group resources by resource type if not already grouped by resource type
            if primary_grouping != GroupingMode::ByResourceType {
                let resource_type_groups =
                    Self::group_by_mode(&primary_resources, &GroupingMode::ByResourceType);

                for (resource_type_key, type_resources) in resource_type_groups {
                    let (type_display, _) = Self::get_display_info(
                        &resource_type_key,
                        &GroupingMode::ByResourceType,
                        &type_resources,
                    );

                    // Create stable node ID for secondary resource type grouping
                    let type_node_id = TreeNode::create_group_node_id(
                        &GroupingMode::ByResourceType,
                        &resource_type_key,
                        Some(&primary_key),
                    );
                    let mut type_node = TreeNode::new(
                        type_node_id,
                        format!("{} ({})", type_display, type_resources.len()),
                        NodeType::ResourceType,
                    );

                    // Add individual resources
                    for resource in type_resources {
                        type_node.add_resource(resource);
                    }

                    primary_node.add_child(type_node);
                }
            } else {
                // When grouping by resource type, create sub-nodes for account/region combinations
                let account_region_groups =
                    Self::group_resources_by_account_region(&primary_resources);

                for ((account_id, region), account_region_resources) in account_region_groups {
                    let sub_display = format!(
                        "{} - {} ({})",
                        account_id,
                        region,
                        account_region_resources.len()
                    );

                    // Create stable node ID for account/region combination under resource type
                    let sub_node_id = TreeNode::create_group_node_id(
                        &GroupingMode::ByAccount,
                        &account_id,
                        Some(&region),
                    );
                    let mut sub_node = TreeNode::new(sub_node_id, sub_display, NodeType::Account);

                    // Add account color if available
                    if let Some(first_resource) = account_region_resources.first() {
                        sub_node = sub_node.with_color(first_resource.account_color);
                    }

                    // Add individual resources to the sub-node
                    for resource in account_region_resources {
                        sub_node.add_resource(resource);
                    }

                    primary_node.add_child(sub_node);
                }
            }

            root.add_child(primary_node);
        }

        // Add total count to root node display name
        let total_resources = filtered_resources.len();
        root.display_name = format!("AWS Resources ({})", total_resources);

        root
    }

    fn group_by_mode(
        resources: &[ResourceEntry],
        grouping: &GroupingMode,
    ) -> HashMap<String, Vec<ResourceEntry>> {
        let mut groups: HashMap<String, Vec<ResourceEntry>> = HashMap::new();

        for resource in resources {
            let key = match grouping {
                GroupingMode::ByAccount => resource.account_id.clone(),
                GroupingMode::ByRegion => resource.region.clone(),
                GroupingMode::ByResourceType => resource.resource_type.clone(),
            };

            groups.entry(key).or_default().push(resource.clone());
        }

        groups
    }

    fn group_resources_by_account_region(
        resources: &[ResourceEntry],
    ) -> HashMap<(String, String), Vec<ResourceEntry>> {
        let mut groups: HashMap<(String, String), Vec<ResourceEntry>> = HashMap::new();

        for resource in resources {
            let key = (resource.account_id.clone(), resource.region.clone());
            groups.entry(key).or_default().push(resource.clone());
        }

        groups
    }

    fn get_display_info(
        key: &str,
        grouping: &GroupingMode,
        resources: &[ResourceEntry],
    ) -> (String, Option<Color32>) {
        match grouping {
            GroupingMode::ByAccount => {
                // For accounts, try to get the display name from the first resource
                // Format: "Display Name - Account ID (count)"
                if let Some(first_resource) = resources.first() {
                    // Try to extract account display name from account color mapping
                    // For now, use account ID as display name but format nicely
                    let display_format = format!("Account {} ({})", key, resources.len());
                    (display_format, Some(first_resource.account_color))
                } else {
                    (format!("Account {} ({})", key, resources.len()), None)
                }
            }
            GroupingMode::ByRegion => {
                // For regions, get the human-readable region name
                // Format: "Region Name - region-id (count)"
                let region_description = Self::get_region_description_static(key);
                let display_format =
                    format!("{} - {} ({})", region_description, key, resources.len());
                if let Some(first_resource) = resources.first() {
                    (display_format, Some(first_resource.region_color))
                } else {
                    (display_format, None)
                }
            }
            GroupingMode::ByResourceType => {
                let display_name = Self::resource_type_to_display_name(key);
                let color = Some(assign_resource_type_color(key));
                (format!("{} ({})", display_name, resources.len()), color)
            }
        }
    }

    fn grouping_to_node_type(grouping: &GroupingMode) -> NodeType {
        match grouping {
            GroupingMode::ByAccount => NodeType::Account,
            GroupingMode::ByRegion => NodeType::Region,
            GroupingMode::ByResourceType => NodeType::ResourceType,
        }
    }

    fn resource_type_to_display_name(resource_type: &str) -> String {
        resource_type.to_string()
    }

    /// Get human-readable description for AWS region (static version for TreeBuilder)
    fn get_region_description_static(region_code: &str) -> &'static str {
        match region_code {
            "us-east-1" => "US East (N. Virginia)",
            "us-east-2" => "US East (Ohio)",
            "us-west-1" => "US West (N. California)",
            "us-west-2" => "US West (Oregon)",
            "eu-west-1" => "Europe (Ireland)",
            "eu-west-2" => "Europe (London)",
            "eu-west-3" => "Europe (Paris)",
            "eu-central-1" => "Europe (Frankfurt)",
            "eu-north-1" => "Europe (Stockholm)",
            "ap-northeast-1" => "Asia Pacific (Tokyo)",
            "ap-northeast-2" => "Asia Pacific (Seoul)",
            "ap-southeast-1" => "Asia Pacific (Singapore)",
            "ap-southeast-2" => "Asia Pacific (Sydney)",
            "ap-south-1" => "Asia Pacific (Mumbai)",
            "sa-east-1" => "South America (São Paulo)",
            "ca-central-1" => "Canada (Central)",
            "af-south-1" => "Africa (Cape Town)",
            "me-south-1" => "Middle East (Bahrain)",
            _ => "Unknown Region",
        }
    }

    fn filter_resources(resources: &[ResourceEntry], search_filter: &str) -> Vec<ResourceEntry> {
        if search_filter.is_empty() {
            return resources.to_vec();
        }

        let matcher = SkimMatcherV2::default();
        let mut scored_resources: Vec<(ResourceEntry, i64)> = Vec::new();

        for resource in resources {
            let mut best_score = None;

            // Try fuzzy matching against multiple fields and take the best score
            let fields_to_search = vec![
                &resource.display_name,
                &resource.resource_type,
                &resource.resource_id,
                &resource.account_id,
                &resource.region,
            ];

            // Check main fields
            for field in fields_to_search {
                if let Some(score) = matcher.fuzzy_match(field, search_filter) {
                    best_score = Some(best_score.map_or(score, |s: i64| s.max(score)));
                }
            }

            // Check tags
            for tag in &resource.tags {
                if let Some(score) = matcher.fuzzy_match(&tag.key, search_filter) {
                    best_score = Some(best_score.map_or(score, |s: i64| s.max(score)));
                }
                if let Some(score) = matcher.fuzzy_match(&tag.value, search_filter) {
                    best_score = Some(best_score.map_or(score, |s: i64| s.max(score)));
                }
            }

            // If we found a match, add it to scored results
            if let Some(score) = best_score {
                scored_resources.push((resource.clone(), score));
            }
        }

        // Sort by score (higher scores first)
        scored_resources.sort_by(|a, b| b.1.cmp(&a.1));

        // Return just the resources
        scored_resources
            .into_iter()
            .map(|(resource, _)| resource)
            .collect()
    }
}

/// Tree renderer for displaying the hierarchical structure in egui using CollapsingHeader
pub struct TreeRenderer {
    // Cache tree structure to prevent unnecessary rebuilds
    cached_tree: Option<TreeNode>,
    cache_key: String, // Hash of resources, grouping, and search filter
    // Resource IDs that need detailed loading
    pub pending_detail_requests: Vec<String>,
    // Resource IDs that failed to load (to prevent infinite retries)
    pub failed_detail_requests: std::collections::HashSet<String>,
}

impl Default for TreeRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeRenderer {
    pub fn new() -> Self {
        Self {
            cached_tree: None,
            cache_key: String::new(),
            pending_detail_requests: Vec::new(),
            failed_detail_requests: std::collections::HashSet::new(),
        }
    }

    /// Get fuzzy match indices for highlighting specific characters
    fn get_fuzzy_match_indices(text: &str, search_filter: &str) -> Option<Vec<usize>> {
        if search_filter.is_empty() {
            return None;
        }

        let matcher = SkimMatcherV2::default();
        if let Some((_, indices)) = matcher.fuzzy_indices(text, search_filter) {
            Some(indices)
        } else {
            None
        }
    }

    /// Render text with character-level fuzzy match highlighting
    fn render_fuzzy_highlighted_text(&self, ui: &mut Ui, text: &str, search_filter: &str) {
        // Only highlight when search is active (3+ characters) but match against the full search term
        if search_filter.len() >= 3 {
            if let Some(indices) = Self::get_fuzzy_match_indices(text, search_filter) {
                // Create a set of highlighted indices for O(1) lookup
                let highlighted_indices: std::collections::HashSet<usize> =
                    indices.into_iter().collect();

                // Render character by character
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0; // No spacing between characters

                    for (i, ch) in text.char_indices() {
                        if highlighted_indices.contains(&i) {
                            // Highlighted character
                            ui.label(
                                RichText::new(ch.to_string())
                                    .background_color(Color32::GREEN)
                                    .color(Color32::BLACK),
                            );
                        } else {
                            // Normal character
                            ui.label(ch.to_string());
                        }
                    }
                });
            } else {
                // No match, render normally
                ui.label(text);
            }
        } else {
            // Search not active yet, render normally
            ui.label(text);
        }
    }

    /// Check if text has a fuzzy match with the search filter
    fn has_fuzzy_match(text: &str, search_filter: &str) -> bool {
        if search_filter.is_empty() {
            return false;
        }

        let matcher = SkimMatcherV2::default();
        matcher.fuzzy_match(text, search_filter).is_some()
    }

    /// Adjust color for current theme - use dark colors for light themes (like Frappe)
    fn adjust_color_for_theme(ui: &Ui, original_color: Color32) -> Color32 {
        // Check if we're using a light theme by looking at the background color
        let bg_color = ui.visuals().window_fill;
        let is_light_theme = Self::is_light_background(bg_color);

        if is_light_theme {
            // For light themes (like Frappe), we need dark colors for readability
            // Try to keep the same hue but make it much darker
            Self::darken_color_preserve_hue(original_color)
        } else {
            // For dark themes, use the original light colors
            original_color
        }
    }

    /// Check if a background color is light (indicating a light theme)
    fn is_light_background(bg_color: Color32) -> bool {
        // Calculate luminosity using standard formula
        let r = bg_color.r() as f32 / 255.0;
        let g = bg_color.g() as f32 / 255.0;
        let b = bg_color.b() as f32 / 255.0;

        // Convert to linear RGB
        let r_linear = if r <= 0.03928 {
            r / 12.92
        } else {
            ((r + 0.055) / 1.055).powf(2.4)
        };
        let g_linear = if g <= 0.03928 {
            g / 12.92
        } else {
            ((g + 0.055) / 1.055).powf(2.4)
        };
        let b_linear = if b <= 0.03928 {
            b / 12.92
        } else {
            ((b + 0.055) / 1.055).powf(2.4)
        };

        // Calculate relative luminosity
        let luminosity = 0.2126 * r_linear + 0.7152 * g_linear + 0.0722 * b_linear;

        // Consider light if luminosity is above 0.5
        luminosity > 0.5
    }

    /// Helper function to ensure vertical lines are disabled
    fn disable_vertical_lines(ui: &mut egui::Ui) {
        ui.visuals_mut().indent_has_left_vline = false;
        // Also update the context style to be sure
        let mut style = (*ui.ctx().style()).clone();
        style.visuals.indent_has_left_vline = false;
        ui.ctx().set_style(style);
    }

    /// Darken a color while trying to preserve its hue
    fn darken_color_preserve_hue(color: Color32) -> Color32 {
        // Convert RGB to HSV to preserve hue
        let r = color.r() as f32 / 255.0;
        let g = color.g() as f32 / 255.0;
        let b = color.b() as f32 / 255.0;

        let max = r.max(g.max(b));
        let min = r.min(g.min(b));
        let delta = max - min;

        // If it's already very dark or grayscale, return black
        if max < 0.3 || delta < 0.1 {
            return Color32::BLACK;
        }

        // Calculate HSV
        let hue = if delta == 0.0 {
            0.0
        } else if max == r {
            60.0 * (((g - b) / delta) % 6.0)
        } else if max == g {
            60.0 * ((b - r) / delta + 2.0)
        } else {
            60.0 * ((r - g) / delta + 4.0)
        };

        let saturation = if max == 0.0 { 0.0 } else { delta / max };

        // Use a much darker value for good contrast on light backgrounds
        let dark_value = 0.3; // Dark but not pure black

        // Convert back to RGB
        let c = dark_value * saturation;
        let x = c * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
        let m = dark_value - c;

        let (r_prime, g_prime, b_prime) = if hue < 60.0 {
            (c, x, 0.0)
        } else if hue < 120.0 {
            (x, c, 0.0)
        } else if hue < 180.0 {
            (0.0, c, x)
        } else if hue < 240.0 {
            (0.0, x, c)
        } else if hue < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        Color32::from_rgb(
            ((r_prime + m) * 255.0) as u8,
            ((g_prime + m) * 255.0) as u8,
            ((b_prime + m) * 255.0) as u8,
        )
    }

    /// Generate a cache key from resources, grouping, and search filter
    /// Only rebuild tree if this key changes
    fn generate_cache_key(
        resources: &[super::state::ResourceEntry],
        primary_grouping: &super::state::GroupingMode,
        search_filter: &str,
    ) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // Hash all resource data for accurate change detection
        resources.len().hash(&mut hasher);
        for resource in resources.iter() {
            resource.resource_id.hash(&mut hasher);
            resource.account_id.hash(&mut hasher);
            resource.region.hash(&mut hasher);
            resource.resource_type.hash(&mut hasher);
        }

        // Hash grouping mode and search filter
        format!("{:?}", primary_grouping).hash(&mut hasher);
        search_filter.hash(&mut hasher);

        format!("{:x}", hasher.finish())
    }

    /// Render tree with caching - only rebuild if data changes
    pub fn render_tree_cached(
        &mut self,
        ui: &mut Ui,
        resources: &[super::state::ResourceEntry],
        primary_grouping: super::state::GroupingMode,
        search_filter: &str,
    ) {
        let new_cache_key = Self::generate_cache_key(resources, &primary_grouping, search_filter);

        // Only rebuild tree if cache key has changed
        if self.cache_key != new_cache_key || self.cached_tree.is_none() {
            tracing::debug!("Tree cache miss - rebuilding tree structure");
            let tree = TreeBuilder::build_tree(resources, primary_grouping, search_filter);
            self.cached_tree = Some(tree);
            self.cache_key = new_cache_key;
        }
        // Remove the cache hit debug message to prevent log flooding

        // Render the cached tree (clone to avoid borrow checker issues)
        if let Some(tree) = self.cached_tree.clone() {
            self.render_node(ui, &tree, 0, search_filter);
        }
    }

    /// Legacy method for backward compatibility
    pub fn render_tree(&mut self, ui: &mut Ui, tree: &TreeNode, search_filter: &str) {
        self.render_node(ui, tree, 0, search_filter);
    }

    fn render_node(&mut self, ui: &mut Ui, node: &TreeNode, depth: usize, search_filter: &str) {
        // LOG: Track tree node rendering to identify structure and ID issues
        tracing::trace!("AWS Explorer: Rendering tree node - Type={:?}, ID={}, DisplayName={}, Depth={}, ChildCount={}, ResourceCount={}",
                       node.node_type, node.id, node.display_name, depth, node.children.len(), node.resource_entries.len());

        // Helper function to render the actual node content
        let mut render_content = |ui: &mut Ui| {
            if !node.is_leaf() {
                // For tree headers, we need to handle highlighting differently due to CollapsingHeader limitations
                let mut final_header = RichText::new(&node.display_name);

                // Apply node color if available, adjusting for light themes
                if let Some(color) = node.color {
                    let adjusted_color = Self::adjust_color_for_theme(ui, color);
                    final_header = final_header.color(adjusted_color);
                }

                // Apply highlighting if fuzzy search matches (only when search is active - 3+ characters)
                if search_filter.len() >= 3
                    && Self::has_fuzzy_match(&node.display_name, search_filter)
                {
                    final_header = final_header
                        .background_color(Color32::GREEN)
                        .color(Color32::BLACK);
                }

                ui.scope(|ui| {
                    Self::disable_vertical_lines(ui);

                    // LOG: Track CollapsingHeader widget ID usage for duplicate detection
                    tracing::trace!("AWS Explorer: Creating tree CollapsingHeader with ID salt: '{}', DisplayName: '{}'",
                                   node.id, node.display_name);

                    egui::CollapsingHeader::new(final_header)
                        .default_open(false)
                        .id_salt(&node.id) // Unique ID for state management
                        .show(ui, |ui| {
                            // Render children
                            for child in &node.children {
                                self.render_node(ui, child, depth + 1, search_filter);
                            }

                            // Render individual resources if this is a leaf node with resources
                            for resource in &node.resource_entries {
                                self.render_resource_node(ui, resource, search_filter);
                            }
                        });
                });
            } else {
                // For leaf nodes, render resources directly
                for resource in &node.resource_entries {
                    self.render_resource_node(ui, resource, search_filter);
                }
            }
        };

        // Apply proper indentation based on depth
        if depth == 0 {
            // Root level - no indentation
            render_content(ui);
        } else {
            // Nested levels - use egui's proper indentation
            ui.indent("node_indent", |ui| {
                render_content(ui);
            });
        }
    }

    fn render_resource_node(&mut self, ui: &mut Ui, resource: &ResourceEntry, search_filter: &str) {
        // Create a unique ID for this resource's tree node - include ALL identifying components for true uniqueness
        // FIXED: Include resource_type and resource_id to prevent duplicates across different resource types
        let resource_node_id = format!(
            "resource_{}:{}:{}:{}:{}",
            resource.resource_type,
            resource.resource_id,
            resource.account_id,
            resource.region,
            resource.display_name
        );

        // LOG: Track resource addition to identify duplicates
        tracing::debug!("AWS Explorer: Adding resource to tree - Region={}, Account={}, ResourceType={}, NodeId={}, ResourceId={}, DisplayName={}",
                       resource.region,
                       resource.account_id,
                       resource.resource_type,
                       resource_node_id,
                       resource.resource_id,
                       resource.display_name);

        // Build the resource text with all the components
        let mut header_parts = Vec::new();

        // Add resource name
        header_parts.push(resource.display_name.clone());

        // Add resource ID
        header_parts.push(format!("({})", resource.resource_id));

        // Add status if available
        if let Some(status) = &resource.status {
            header_parts.push(format!("[{}]", status));
        }

        // Add age indicator
        let age_text = resource.get_age_display();
        let is_stale = resource.is_stale(15);
        if is_stale {
            header_parts.push(format!("⚠ {}", age_text));
        } else {
            header_parts.push(age_text);
        }

        let header_text = header_parts.join(" ");

        // Use vertical layout to separate header from JSON content
        ui.vertical(|ui| {
            // Header with proper order: arrow, account tag, region tag, then resource info
            let response = ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0; // Remove spacing between elements

                // Arrow first - Use CollapsingHeader for expand/collapse functionality
                let response = ui.scope(|ui| {
                    Self::disable_vertical_lines(ui);

                    // LOG: Track resource CollapsingHeader widget ID usage for duplicate detection
                    tracing::warn!("AWS Explorer: Creating resource CollapsingHeader with ID salt: '{}', ResourceType: '{}', ResourceId: '{}', DisplayName: '{}'",
                                  resource_node_id, resource.resource_type, resource.resource_id, resource.display_name);

                    egui::CollapsingHeader::new("")
                        .id_salt(&resource_node_id)
                        .default_open(false)
                        .show_background(false)
                        .show(ui, |_ui| {
                            // Empty - content will be rendered below
                        })
                }).inner;

                // Account and region tags directly after arrow (no space)
                self.render_account_tag(ui, &resource.account_id, resource.account_color);
                ui.add_space(4.0);
                self.render_region_tag(ui, &resource.region, resource.region_color);
                ui.add_space(8.0);

                // Render the highlighted resource info text
                self.render_fuzzy_highlighted_text(ui, &header_text, search_filter);

                response
            }).inner;

            // Only request detailed properties when the header is actually expanded AND clicked
            if response.header_response.clicked() && response.openness > 0.0 && resource.detailed_properties.is_none() {
                let resource_key = format!("{}:{}:{}", resource.account_id, resource.region, resource.resource_id);
                // Only request if not already pending and not previously failed
                if !self.pending_detail_requests.contains(&resource_key)
                    && !self.failed_detail_requests.contains(&resource_key) {
                    self.pending_detail_requests.push(resource_key);
                    tracing::info!("🔄 Requesting detailed properties for: {}", resource.display_name);
                }
            }

            // Show JSON tree as indented child below the header when expanded
            if response.openness > 0.0 {
                ui.indent("json_indent", |ui| {
                    // Add additional indentation to make it clearly a child
                    ui.indent("json_child_indent", |ui| {
                        self.render_json_tree(ui, resource);
                    });
                });
            }
        });
    }

    /// Render JSON tree viewer for detailed resource properties
    fn render_json_tree(&mut self, ui: &mut Ui, resource: &ResourceEntry) {
        // Add egui_json_tree import at the top if needed
        use egui_json_tree::JsonTree;

        // Create unique IDs for all components to prevent widget ID warnings
        let resource_id = &resource.resource_id;
        let resize_id = format!("json_resize_{}", resource_id);
        let scroll_id = format!("json_scroll_{}", resource_id);
        let search_id = format!("json_search_{}", resource_id);

        // Use egui's built-in Resize widget with fixed sizing behavior
        egui::Resize::default()
            .id_salt(&resize_id) // Unique ID for resize widget
            .default_size([800.0, 300.0]) // Default size - doubled width
            .min_size([300.0, 150.0])
            .resizable(true)
            .show(ui, |ui| {
                // Use available height minus space for header/footer to maintain consistent scroll area size
                let available_height = ui.available_height();
                let header_footer_height = 90.0; // Estimated space for header + footer + separators
                let scroll_height = (available_height - header_footer_height).max(100.0);

                ui.vertical(|ui| {
                    // Header with search functionality
                    ui.horizontal(|ui| {
                        ui.label("🔍 Search:");
                        // Use unique ID for search text field
                        ui.add_enabled_ui(false, |ui| {
                            ui.add(egui::TextEdit::singleline(&mut String::new())
                                .id_salt(&search_id)
                                .hint_text("Search JSON (coming soon)"));
                        });
                    });

                    ui.separator();

                    // JSON Tree viewer with fixed size scroll area
                    egui::ScrollArea::vertical()
                        .id_salt(&scroll_id) // Unique ID for scroll area
                        .auto_shrink([false, false]) // Don't auto-shrink - maintain container size
                        .max_height(scroll_height) // Fixed height based on container
                        .show(ui, |ui| {
                            if resource.detailed_properties.is_some() {
                                // Show detailed properties if available
                                let json_data = resource.get_display_properties();
                                JsonTree::new(format!("resource_json_detailed_{}", resource_id), json_data)
                                    .default_expand(egui_json_tree::DefaultExpand::ToLevel(3))
                                    .show(ui);
                            } else {
                                // Show loading state, failed state
                                let resource_key = format!("{}:{}:{}", resource.account_id, resource.region, resource.resource_id);
                                if self.failed_detail_requests.contains(&resource_key) {
                                    ui.horizontal(|ui| {
                                        ui.colored_label(Color32::from_rgb(255, 165, 0), "⚠");
                                        ui.label("Detailed properties not available for this resource type");
                                    });
                                    // Show basic list data
                                    let json_data = resource.get_display_properties();
                                    JsonTree::new(format!("resource_json_basic_{}", resource_id), json_data)
                                        .default_expand(egui_json_tree::DefaultExpand::ToLevel(2))
                                        .show(ui);
                                } else if self.pending_detail_requests.contains(&resource_key) {
                                    ui.horizontal(|ui| {
                                        ui.spinner();
                                        ui.label("Loading detailed properties...");
                                    });
                                } else {
                                    // Show basic list data in the meantime
                                    let json_data = resource.get_display_properties();
                                    JsonTree::new(format!("resource_json_fallback_{}", resource_id), json_data)
                                        .default_expand(egui_json_tree::DefaultExpand::ToLevel(2))
                                        .show(ui);
                                }
                            }
                        });

                    ui.separator();

                    // Footer with actions
                    ui.horizontal(|ui| {
                        if ui.small_button("📋 Copy JSON").clicked() {
                            let formatted_json = serde_json::to_string_pretty(resource.get_display_properties())
                                .unwrap_or_else(|_| "Error formatting JSON".to_string());
                            ui.ctx().copy_text(formatted_json);
                        }

                        ui.add_space(10.0);

                        // Show data freshness
                        if resource.detailed_properties.is_some() {
                            ui.label("📄 Detailed data loaded");
                        } else {
                            let resource_key = format!("{}:{}:{}", resource.account_id, resource.region, resource.resource_id);
                            if self.failed_detail_requests.contains(&resource_key) {
                                ui.colored_label(Color32::from_rgb(255, 165, 0), "⚠ Detailed loading not supported");
                            } else if self.pending_detail_requests.contains(&resource_key) {
                                ui.label("🔄 Loading detailed data...");
                            }
                        }
                    });
                });
            });
    }

    /// Render an account tag with colored background
    fn render_account_tag(&self, ui: &mut Ui, account_id: &str, account_color: Color32) {
        let text_color = get_contrasting_text_color(account_color);

        // Create shortened account display (last 4 digits)
        let account_display = if account_id.len() >= 4 {
            format!("...{}", &account_id[account_id.len() - 4..])
        } else {
            account_id.to_string()
        };

        // Calculate text size
        let font_size = 10.0;
        let text_galley = ui.fonts(|fonts| {
            fonts.layout_no_wrap(
                account_display.clone(),
                egui::FontId::monospace(font_size),
                text_color,
            )
        });

        // Add padding to the text size
        let padding = egui::vec2(6.0, 2.0);
        let desired_size = text_galley.size() + 2.0 * padding;

        // Allocate space for the tag
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            // Draw rounded rectangle background
            ui.painter().rect_filled(
                rect,
                3.0, // corner radius
                account_color,
            );

            // Draw text centered in the rect
            let text_pos = rect.center() - text_galley.size() / 2.0;
            ui.painter().galley(text_pos, text_galley, text_color);
        }

        // Show tooltip with full account ID on hover
        response.on_hover_text(format!("Account: {}", account_id));
    }

    /// Render a region tag with colored background
    fn render_region_tag(&self, ui: &mut Ui, region_code: &str, region_color: Color32) {
        let text_color = get_contrasting_text_color(region_color);

        // Calculate text size
        let font_size = 10.0;
        let text_galley = ui.fonts(|fonts| {
            fonts.layout_no_wrap(
                region_code.to_string(),
                egui::FontId::monospace(font_size),
                text_color,
            )
        });

        // Add padding to the text size
        let padding = egui::vec2(6.0, 2.0);
        let desired_size = text_galley.size() + 2.0 * padding;

        // Allocate space for the tag
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            // Draw rounded rectangle background
            ui.painter().rect_filled(
                rect,
                3.0, // corner radius
                region_color,
            );

            // Draw text centered in the rect
            let text_pos = rect.center() - text_galley.size() / 2.0;
            ui.painter().galley(text_pos, text_galley, text_color);
        }

        // Show tooltip with region description on hover
        let region_description = self.get_region_description(region_code);
        response.on_hover_text(format!("Region: {} ({})", region_code, region_description));
    }

    /// Get human-readable description for AWS region
    fn get_region_description(&self, region_code: &str) -> &'static str {
        match region_code {
            "us-east-1" => "US East (N. Virginia)",
            "us-east-2" => "US East (Ohio)",
            "us-west-1" => "US West (N. California)",
            "us-west-2" => "US West (Oregon)",
            "eu-west-1" => "Europe (Ireland)",
            "eu-west-2" => "Europe (London)",
            "eu-west-3" => "Europe (Paris)",
            "eu-central-1" => "Europe (Frankfurt)",
            "eu-north-1" => "Europe (Stockholm)",
            "ap-northeast-1" => "Asia Pacific (Tokyo)",
            "ap-northeast-2" => "Asia Pacific (Seoul)",
            "ap-southeast-1" => "Asia Pacific (Singapore)",
            "ap-southeast-2" => "Asia Pacific (Sydney)",
            "ap-south-1" => "Asia Pacific (Mumbai)",
            "sa-east-1" => "South America (São Paulo)",
            "ca-central-1" => "Canada (Central)",
            "af-south-1" => "Africa (Cape Town)",
            "me-south-1" => "Middle East (Bahrain)",
            _ => "Unknown Region",
        }
    }
}
