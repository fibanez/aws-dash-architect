use super::{colors::*, state::*};
use crate::app::data_plane::cloudtrail_events::has_cloudtrail_support;
use crate::app::data_plane::cloudwatch_logs::{get_log_group_name, has_cloudwatch_logs};
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

        // Separate parent resources from child resources
        // Child resources will be attached to their parents, not shown at top level
        let parent_resources: Vec<ResourceEntry> = filtered_resources
            .iter()
            .filter(|r| !r.is_child_resource)
            .cloned()
            .collect();

        let mut root = TreeNode::new(
            "root".to_string(),
            "AWS Resources".to_string(),
            NodeType::Resource,
        );

        // Special handling for hierarchical tag grouping
        if let GroupingMode::ByTagHierarchy(tag_keys) = &primary_grouping {
            if !tag_keys.is_empty() {
                Self::build_tag_hierarchy_tree(
                    &mut root,
                    &parent_resources,
                    tag_keys,
                    &filtered_resources,
                );

                // Add total count to root node display name
                let total_resources = filtered_resources.len();
                root.display_name = format!("AWS Resources ({})", total_resources);

                return root;
            }
        }

        // Special handling for hierarchical property grouping
        if let GroupingMode::ByPropertyHierarchy(property_paths) = &primary_grouping {
            if !property_paths.is_empty() {
                Self::build_property_hierarchy_tree(
                    &mut root,
                    &parent_resources,
                    property_paths,
                    &filtered_resources,
                );

                // Add total count to root node display name
                let total_resources = filtered_resources.len();
                root.display_name = format!("AWS Resources ({})", total_resources);

                return root;
            }
        }

        // Group by primary grouping (only parent resources)
        let primary_groups = Self::group_by_mode(&parent_resources, &primary_grouping);

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
                    for resource in &type_resources {
                        type_node.add_resource(resource.clone());
                    }

                    // Attach child resources to their parent resources
                    Self::attach_child_resources(&mut type_node, &filtered_resources);

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
                    for resource in &account_region_resources {
                        sub_node.add_resource(resource.clone());
                    }

                    // Attach child resources to their parent resources
                    Self::attach_child_resources(&mut sub_node, &filtered_resources);

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

    /// Attach child resources as tree nodes under their parent resources
    /// This creates a hierarchical structure for resources with parent-child relationships
    fn attach_child_resources(parent_node: &mut TreeNode, all_resources: &[ResourceEntry]) {
        // Iterate through all resource entries in this node
        // We need to clone the resource_entries to avoid borrowing issues
        let parent_resources = parent_node.resource_entries.clone();

        for parent_resource in &parent_resources {
            // Find all child resources for this parent
            let children: Vec<ResourceEntry> = all_resources
                .iter()
                .filter(|r| {
                    r.is_child_resource
                        && r.parent_resource_id.as_ref() == Some(&parent_resource.resource_id)
                        && r.parent_resource_type.as_ref() == Some(&parent_resource.resource_type)
                })
                .cloned()
                .collect();

            if !children.is_empty() {
                // Create a child node for this parent resource with its children
                // Group children by resource type
                let mut child_groups: HashMap<String, Vec<ResourceEntry>> = HashMap::new();
                for child in &children {
                    child_groups
                        .entry(child.resource_type.clone())
                        .or_default()
                        .push(child.clone());
                }

                // For each child resource type, create a sub-node
                for (child_type, child_resources) in child_groups {
                    let child_type_display = child_type
                        .strip_prefix("AWS::")
                        .and_then(|s| s.split("::").last())
                        .unwrap_or(&child_type);

                    let child_node_id = format!(
                        "child:{}:{}:{}",
                        parent_resource.resource_id, parent_resource.resource_type, child_type
                    );

                    let mut child_node = TreeNode::new(
                        child_node_id,
                        format!("{} ({})", child_type_display, child_resources.len()),
                        NodeType::ResourceType,
                    );

                    // Add child resources to this node
                    for child_resource in &child_resources {
                        child_node.add_resource(child_resource.clone());
                    }

                    // Recursively attach grandchildren
                    Self::attach_child_resources(&mut child_node, all_resources);

                    // Add the child node to the parent node
                    parent_node.add_child(child_node);
                }
            }
        }
    }

    /// Build a hierarchical tree structure based on multiple tag keys
    ///
    /// This method recursively groups resources by tag keys in the specified order.
    /// For example, with tag_keys = ["Environment", "Team", "Project"]:
    /// - Level 1: Group by Environment (Production, Staging, etc.)
    /// - Level 2: Under each Environment, group by Team (Backend, Frontend, etc.)
    /// - Level 3: Under each Team, group by Project (API, WebUI, etc.)
    ///
    /// Resources without a tag at any level are grouped under "No {TagKey}" (e.g., "No Environment")
    fn build_tag_hierarchy_tree(
        parent_node: &mut TreeNode,
        resources: &[ResourceEntry],
        tag_keys: &[String],
        all_resources: &[ResourceEntry],
    ) {
        if tag_keys.is_empty() || resources.is_empty() {
            return;
        }

        let current_tag_key = &tag_keys[0];
        let remaining_tag_keys = &tag_keys[1..];

        // Group resources by the current tag key
        let mut groups: HashMap<String, Vec<ResourceEntry>> = HashMap::new();

        for resource in resources {
            let tag_value = resource
                .tags
                .iter()
                .find(|tag| &tag.key == current_tag_key)
                .map(|tag| tag.value.clone())
                .unwrap_or_else(|| format!("No {}", current_tag_key));

            groups.entry(tag_value).or_default().push(resource.clone());
        }

        // Sort groups alphabetically, but put "No {tag}" groups at the end
        let mut sorted_keys: Vec<String> = groups.keys().cloned().collect();
        let no_tag_label = format!("No {}", current_tag_key);
        sorted_keys.sort_by(|a, b| match (a.as_str(), b.as_str()) {
            (a_val, b_val) if a_val == no_tag_label && b_val == no_tag_label => {
                std::cmp::Ordering::Equal
            }
            (a_val, _) if a_val == no_tag_label => std::cmp::Ordering::Greater,
            (_, b_val) if b_val == no_tag_label => std::cmp::Ordering::Less,
            _ => a.cmp(b),
        });

        for tag_value in sorted_keys {
            let group_resources = groups.get(&tag_value).unwrap();

            // Create display name for this tag group
            let display_name = if tag_value == no_tag_label {
                format!("{} ({} resources)", no_tag_label, group_resources.len())
            } else {
                format!(
                    "{}: {} ({})",
                    current_tag_key,
                    tag_value,
                    group_resources.len()
                )
            };

            // Create stable node ID for this tag group
            let node_id = format!("tag:{}:{}", current_tag_key, tag_value);
            let mut tag_node = TreeNode::new(node_id, display_name, NodeType::Account);

            // Use consistent color for tag groups
            let tag_color = if tag_value == no_tag_label {
                Color32::from_rgb(150, 150, 150) // Gray for missing tag
            } else {
                Color32::from_rgb(100, 150, 200) // Blue for tagged
            };
            tag_node = tag_node.with_color(tag_color);

            // If there are more tag keys, recursively build the hierarchy
            if !remaining_tag_keys.is_empty() {
                Self::build_tag_hierarchy_tree(
                    &mut tag_node,
                    group_resources,
                    remaining_tag_keys,
                    all_resources,
                );
            } else {
                // This is the last level - group by resource type and add resources
                let resource_type_groups =
                    Self::group_by_mode(group_resources, &GroupingMode::ByResourceType);

                for (resource_type, type_resources) in resource_type_groups {
                    let type_display = Self::resource_type_to_display_name(&resource_type);
                    let type_node_id = format!("{}:type:{}", tag_node.id, resource_type);

                    let mut type_node = TreeNode::new(
                        type_node_id,
                        format!("{} ({})", type_display, type_resources.len()),
                        NodeType::ResourceType,
                    );

                    // Add individual resources
                    for resource in &type_resources {
                        type_node.add_resource(resource.clone());
                    }

                    // Attach child resources to their parent resources
                    Self::attach_child_resources(&mut type_node, all_resources);

                    tag_node.add_child(type_node);
                }
            }

            parent_node.add_child(tag_node);
        }
    }

    /// Build a hierarchical tree structure based on multiple property paths
    ///
    /// This method recursively groups resources by property values in the specified order.
    /// For example, with property_paths = ["raw_properties.State", "raw_properties.InstanceType"]:
    /// - Level 1: Group by State (running, stopped, etc.)
    /// - Level 2: Under each State, group by InstanceType (t2.micro, m5.large, etc.)
    ///
    /// Resources without a property at any level are grouped under "No {PropertyName}" (e.g., "No State")
    fn build_property_hierarchy_tree(
        parent_node: &mut TreeNode,
        resources: &[ResourceEntry],
        property_paths: &[String],
        all_resources: &[ResourceEntry],
    ) {
        if property_paths.is_empty() || resources.is_empty() {
            return;
        }

        let current_property_path = &property_paths[0];
        let remaining_property_paths = &property_paths[1..];

        tracing::info!("=== Property Hierarchy Grouping ===");
        tracing::info!("Property path: {}", current_property_path);
        tracing::info!("Resources to group: {}", resources.len());

        // Log first resource structure for debugging
        if let Some(first_resource) = resources.first() {
            tracing::info!("First resource type: {}", first_resource.resource_type);
            tracing::info!(
                "First resource properties JSON: {}",
                serde_json::to_string_pretty(&first_resource.properties)
                    .unwrap_or_else(|_| "ERROR".to_string())
            );
        }

        // Group resources by the current property value
        let mut groups: HashMap<String, Vec<ResourceEntry>> = HashMap::new();

        for resource in resources {
            let property_value =
                Self::extract_property_value_from_resource(resource, current_property_path)
                    .unwrap_or_else(|| {
                        format!("No {}", Self::property_display_name(current_property_path))
                    });

            groups
                .entry(property_value)
                .or_default()
                .push(resource.clone());
        }

        tracing::info!("Grouped into {} distinct values", groups.len());
        for (value, res) in &groups {
            tracing::info!("  '{}': {} resources", value, res.len());
        }

        // Sort property values alphabetically
        let mut sorted_values: Vec<String> = groups.keys().cloned().collect();
        sorted_values.sort();

        for property_value in sorted_values {
            let group_resources = groups.get(&property_value).unwrap();

            // Create display name for this property group
            let display_name = format!(
                "{}: {} ({})",
                Self::property_display_name(current_property_path),
                property_value,
                group_resources.len()
            );

            // Create stable node ID for this property group
            let node_id = format!("property:{}:{}", current_property_path, property_value);
            let mut property_node = TreeNode::new(node_id, display_name, NodeType::Account);

            // Use color for property groups
            let property_color = Color32::from_rgb(100, 200, 150); // Green for properties
            property_node = property_node.with_color(property_color);

            // If there are more property paths, recursively build the hierarchy
            if !remaining_property_paths.is_empty() {
                Self::build_property_hierarchy_tree(
                    &mut property_node,
                    group_resources,
                    remaining_property_paths,
                    all_resources,
                );
            } else {
                // This is the last level - group by resource type and add resources
                let resource_type_groups =
                    Self::group_by_mode(group_resources, &GroupingMode::ByResourceType);

                for (resource_type, type_resources) in resource_type_groups {
                    let type_display = Self::resource_type_to_display_name(&resource_type);
                    let type_node_id = format!("{}:type:{}", property_node.id, resource_type);

                    let mut type_node = TreeNode::new(
                        type_node_id,
                        format!("{} ({})", type_display, type_resources.len()),
                        NodeType::ResourceType,
                    );

                    // Add resources to the type node
                    for resource in type_resources {
                        type_node.add_resource(resource.clone());
                    }

                    // Attach child resources to their parent resources
                    Self::attach_child_resources(&mut type_node, all_resources);

                    property_node.add_child(type_node);
                }
            }

            parent_node.add_child(property_node);
        }
    }

    /// Get a display-friendly name for a property path (show full path)
    fn property_display_name(property_path: &str) -> String {
        property_path.to_string()
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
                GroupingMode::ByTag(tag_key) => {
                    // Group by single tag value
                    resource
                        .tags
                        .iter()
                        .find(|tag| &tag.key == tag_key)
                        .map(|tag| tag.value.clone())
                        .unwrap_or_else(|| format!("No {}", tag_key))
                }
                GroupingMode::ByTagHierarchy(_) => {
                    // Tag hierarchy grouping requires special handling
                    // For now, fall back to account grouping
                    // Future enhancement: full tag hierarchy implementation
                    resource.account_id.clone()
                }
                GroupingMode::ByProperty(property_path) => {
                    // Group by single property value
                    Self::extract_property_value_from_resource(resource, property_path)
                        .unwrap_or_else(|| "(not set)".to_string())
                }
                GroupingMode::ByPropertyHierarchy(_) => {
                    // Property hierarchy grouping requires special handling
                    // For now, fall back to account grouping
                    resource.account_id.clone()
                }
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
            GroupingMode::ByTag(tag_key) => {
                // For tag grouping, display tag value
                let no_tag_label = format!("No {}", tag_key);
                let display_name = if key == no_tag_label {
                    format!("{} ({} resources)", no_tag_label, resources.len())
                } else {
                    format!("{}: {} ({})", tag_key, key, resources.len())
                };
                // Use a consistent color for tag groups
                let color = if key == no_tag_label {
                    Some(Color32::from_rgb(150, 150, 150)) // Gray for missing tag
                } else {
                    Some(Color32::from_rgb(100, 150, 200)) // Blue for tagged
                };
                (display_name, color)
            }
            GroupingMode::ByTagHierarchy(_) => {
                // Future enhancement: tag hierarchy implementation
                // For now, use a generic display
                (format!("{} ({})", key, resources.len()), None)
            }
            GroupingMode::ByProperty(property_path) => {
                // For property grouping, display property value
                let not_set_label = "(not set)";
                let display_name = if key == not_set_label {
                    format!("{} ({} resources)", not_set_label, resources.len())
                } else {
                    // Extract last segment of property path for display
                    let property_name = property_path
                        .split('.')
                        .next_back()
                        .unwrap_or(property_path);
                    format!("{}: {} ({})", property_name, key, resources.len())
                };
                // Use a consistent color for property groups
                let color = if key == not_set_label {
                    Some(Color32::from_rgb(150, 150, 150)) // Gray for missing property
                } else {
                    Some(Color32::from_rgb(150, 100, 200)) // Purple for properties
                };
                (display_name, color)
            }
            GroupingMode::ByPropertyHierarchy(_) => {
                // Property hierarchy will be implemented later
                // For now, use a generic display
                (format!("{} ({})", key, resources.len()), None)
            }
        }
    }

    fn grouping_to_node_type(grouping: &GroupingMode) -> NodeType {
        match grouping {
            GroupingMode::ByAccount => NodeType::Account,
            GroupingMode::ByRegion => NodeType::Region,
            GroupingMode::ByResourceType => NodeType::ResourceType,
            GroupingMode::ByTag(_) => NodeType::Account, // Temporary placeholder
            GroupingMode::ByTagHierarchy(_) => NodeType::Account, // Temporary placeholder
            GroupingMode::ByProperty(_) => NodeType::Account, // Temporary placeholder
            GroupingMode::ByPropertyHierarchy(_) => NodeType::Account, // Temporary placeholder
        }
    }

    fn resource_type_to_display_name(resource_type: &str) -> String {
        resource_type.to_string()
    }

    /// Get human-readable description for AWS region (static version for TreeBuilder)
    fn get_region_description_static(region_code: &str) -> &'static str {
        match region_code {
            "Global" => "Global",
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

    /// Extract a property value from a ResourceEntry using dot notation
    ///
    /// This searches across multiple property fields in order of preference:
    /// 1. detailed_properties (most complete)
    /// 2. raw_properties (original AWS response)
    /// 3. properties (minimal normalized data)
    ///
    /// Property paths can have prefixes like "raw_properties.State" or just "State"
    fn extract_property_value_from_resource(
        resource: &ResourceEntry,
        property_path: &str,
    ) -> Option<String> {
        tracing::debug!(
            "Extracting property '{}' from resource {}",
            property_path,
            resource.resource_type
        );

        // Determine which field to search and the actual path within that field
        let (search_field, actual_path) = if property_path.starts_with("detailed_properties.") {
            (
                "detailed",
                property_path.strip_prefix("detailed_properties.").unwrap(),
            )
        } else if property_path.starts_with("raw_properties.") {
            (
                "raw",
                property_path.strip_prefix("raw_properties.").unwrap(),
            )
        } else if property_path.starts_with("properties.") {
            (
                "properties",
                property_path.strip_prefix("properties.").unwrap(),
            )
        } else {
            // No prefix - search in order: detailed → raw → properties
            ("auto", property_path)
        };

        tracing::debug!(
            "  Search strategy: field='{}', path='{}'",
            search_field,
            actual_path
        );

        // Try extraction based on strategy
        match search_field {
            "detailed" => {
                if let Some(ref detailed) = resource.detailed_properties {
                    Self::extract_from_json(detailed, actual_path)
                } else {
                    tracing::debug!("  detailed_properties not available");
                    None
                }
            }
            "raw" => Self::extract_from_json(&resource.raw_properties, actual_path),
            "properties" => Self::extract_from_json(&resource.properties, actual_path),
            "auto" => {
                // Try detailed first, then raw, then properties
                if let Some(ref detailed) = resource.detailed_properties {
                    if let Some(value) = Self::extract_from_json(detailed, actual_path) {
                        tracing::debug!("  Found in detailed_properties");
                        return Some(value);
                    }
                }

                if let Some(value) = Self::extract_from_json(&resource.raw_properties, actual_path)
                {
                    tracing::debug!("  Found in raw_properties");
                    return Some(value);
                }

                if let Some(value) = Self::extract_from_json(&resource.properties, actual_path) {
                    tracing::debug!("  Found in properties");
                    return Some(value);
                }

                tracing::debug!("  NOT FOUND in any property field");
                None
            }
            _ => None,
        }
    }

    /// Extract a value from JSON using dot notation
    fn extract_from_json(json: &serde_json::Value, property_path: &str) -> Option<String> {
        let segments: Vec<&str> = property_path.split('.').collect();
        let mut current = json;

        // Navigate through the JSON structure
        for segment in segments {
            match current {
                serde_json::Value::Object(map) => {
                    current = map.get(segment)?;
                }
                _ => return None,
            }
        }

        // Convert the final value to a string
        match current {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Number(n) => Some(n.to_string()),
            serde_json::Value::Bool(b) => Some(b.to_string()),
            serde_json::Value::Null => None,
            _ => Some(current.to_string()),
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
    // Tag badge clicks (for adding filters)
    pub pending_tag_clicks: Vec<super::state::TagClickAction>,
    // Pending actions to communicate with main app (e.g., open CloudWatch Logs)
    pub pending_explorer_actions: Vec<super::ResourceExplorerAction>,
    // Tag badge support
    badge_selector: Option<super::tag_badges::BadgeSelector>,
    tag_popularity: Option<super::tag_badges::TagPopularityTracker>,
    // Flag to track if we're currently rebuilding (for logging debouncing)
    is_rebuilding: bool,
    // JSON tree viewer state per resource
    json_expand_levels: std::collections::HashMap<String, u8>,
    json_search_terms: std::collections::HashMap<String, String>,
    // Track which resource names are expanded (not truncated)
    expanded_names: std::collections::HashSet<String>,
    // Phase 2 enrichment status (set by parent before rendering)
    pub phase2_in_progress: bool,
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
            pending_tag_clicks: Vec::new(),
            pending_explorer_actions: Vec::new(),
            badge_selector: None,
            tag_popularity: None,
            is_rebuilding: false,
            json_expand_levels: std::collections::HashMap::new(),
            json_search_terms: std::collections::HashMap::new(),
            expanded_names: std::collections::HashSet::new(),
            phase2_in_progress: false,
        }
    }

    /// Get the expand level for a resource (default: 3)
    fn get_expand_level(&self, resource_id: &str) -> u8 {
        *self.json_expand_levels.get(resource_id).unwrap_or(&3)
    }

    /// Get the search term for a resource
    fn get_search_term(&self, resource_id: &str) -> String {
        self.json_search_terms
            .get(resource_id)
            .cloned()
            .unwrap_or_default()
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

    /// Generate a cache key from resources, grouping, search filter, and enrichment version
    /// Only rebuild tree if this key changes
    fn generate_cache_key(
        resources: &[super::state::ResourceEntry],
        primary_grouping: &super::state::GroupingMode,
        search_filter: &str,
        enrichment_version: u64,
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

        // Hash enrichment version to invalidate cache when Phase 2 updates properties
        enrichment_version.hash(&mut hasher);

        format!("{:x}", hasher.finish())
    }

    /// Render tree with caching - only rebuild if data changes
    pub fn render_tree_cached(
        &mut self,
        ui: &mut Ui,
        resources: &[super::state::ResourceEntry],
        primary_grouping: super::state::GroupingMode,
        search_filter: &str,
        badge_selector: &super::tag_badges::BadgeSelector,
        tag_popularity: &super::tag_badges::TagPopularityTracker,
        enrichment_version: u64,
    ) {
        // Update badge support (clone to store in renderer)
        self.badge_selector = Some(badge_selector.clone());
        self.tag_popularity = Some(tag_popularity.clone());

        let new_cache_key = Self::generate_cache_key(resources, &primary_grouping, search_filter, enrichment_version);

        // Only rebuild tree if cache key has changed
        if self.cache_key != new_cache_key || self.cached_tree.is_none() {
            tracing::debug!("Tree cache miss - rebuilding tree structure");
            self.is_rebuilding = true; // Enable verbose logging during rebuild
            let tree = TreeBuilder::build_tree(resources, primary_grouping, search_filter);
            self.cached_tree = Some(tree);
            self.cache_key = new_cache_key;
        } else {
            self.is_rebuilding = false; // Disable verbose logging for cached renders
        }

        // Render the cached tree (clone to avoid borrow checker issues)
        if let Some(tree) = self.cached_tree.clone() {
            self.render_node(ui, &tree, 0, search_filter);
        }

        // Reset rebuild flag after rendering
        self.is_rebuilding = false;
    }

    /// Legacy method for backward compatibility
    pub fn render_tree(&mut self, ui: &mut Ui, tree: &TreeNode, search_filter: &str) {
        self.render_node(ui, tree, 0, search_filter);
    }

    fn render_node(&mut self, ui: &mut Ui, node: &TreeNode, depth: usize, search_filter: &str) {
        // LOG: Track tree node rendering only during rebuild to avoid flooding
        if self.is_rebuilding {
            tracing::trace!("AWS Explorer: Rendering tree node - Type={:?}, ID={}, DisplayName={}, Depth={}, ChildCount={}, ResourceCount={}",
                           node.node_type, node.id, node.display_name, depth, node.children.len(), node.resource_entries.len());
        }

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

                    // LOG: Track CollapsingHeader widget ID usage only during rebuild
                    if self.is_rebuilding {
                        tracing::trace!("AWS Explorer: Creating tree CollapsingHeader with ID salt: '{}', DisplayName: '{}'",
                                       node.id, node.display_name);
                    }

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

    fn render_resource_node(
        &mut self,
        ui: &mut Ui,
        resource: &ResourceEntry,
        _search_filter: &str,
    ) {
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

        // LOG: Track resource addition only during rebuild to avoid flooding
        if self.is_rebuilding {
            tracing::debug!("AWS Explorer: Adding resource to tree - Region={}, Account={}, ResourceType={}, NodeId={}, ResourceId={}, DisplayName={}",
                           resource.region,
                           resource.account_id,
                           resource.resource_type,
                           resource_node_id,
                           resource.resource_id,
                           resource.display_name);
        }

        // Build the resource name and ID for the colored tag
        let resource_name_id = format!("{} ({})", resource.display_name, resource.resource_id);

        // Build status and age info to display separately after the tag
        let mut additional_info = Vec::new();

        // Add status if available
        if let Some(status) = &resource.status {
            additional_info.push(format!("[{}]", status));
        }

        // Add age indicator
        let age_text = resource.get_age_display();
        let is_stale = resource.is_stale(15);
        if is_stale {
            additional_info.push(format!("⚠ {}", age_text));
        } else {
            additional_info.push(age_text);
        }

        // Use vertical layout to separate header from JSON content
        ui.vertical(|ui| {
            // Header with proper order: arrow, account tag, region tag, then resource info
            let response = ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0; // Remove spacing between elements

                // Arrow first - Use CollapsingHeader for expand/collapse functionality
                let response = ui.scope(|ui| {
                    Self::disable_vertical_lines(ui);

                    // LOG: Track resource CollapsingHeader widget ID only during rebuild
                    if self.is_rebuilding {
                        tracing::debug!("AWS Explorer: Creating resource CollapsingHeader with ID salt: '{}', ResourceType: '{}', ResourceId: '{}', DisplayName: '{}'",
                                      resource_node_id, resource.resource_type, resource.resource_id, resource.display_name);
                    }

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
                ui.add_space(4.0);

                // Resource type short tag (e.g., "LAMBDA", "SEC-GROUP")
                self.render_resource_type_short_tag(ui, &resource.resource_type);
                ui.add_space(8.0);

                // Check if this resource's name is expanded (not truncated)
                let is_name_expanded = self.expanded_names.contains(&resource_node_id);

                // Render the resource type tag with colored background based on resource type
                // The color is determined by resource_type (e.g., "AWS::EC2::Instance")
                // but the interior displays only the resource name and ID
                let tag_response = self.render_resource_type_tag(
                    ui,
                    &resource_name_id,
                    &resource.resource_type,
                    is_name_expanded,
                );

                // Handle left-click to toggle expanded/collapsed name
                if tag_response.clicked() {
                    if is_name_expanded {
                        self.expanded_names.remove(&resource_node_id);
                    } else {
                        self.expanded_names.insert(resource_node_id.clone());
                    }
                }

                // Handle right-click context menu for copy options
                tag_response.context_menu(|ui| {
                    if ui.button("Copy Name").clicked() {
                        ui.ctx().copy_text(resource.display_name.clone());
                        ui.close();
                    }
                    if ui.button("Copy ID").clicked() {
                        ui.ctx().copy_text(resource.resource_id.clone());
                        ui.close();
                    }
                    if ui.button("Copy Name (ID)").clicked() {
                        ui.ctx().copy_text(resource_name_id.clone());
                        ui.close();
                    }
                    if let Some(arn) = resource.properties.get("Arn").and_then(|v| v.as_str()) {
                        if ui.button("Copy ARN").clicked() {
                            ui.ctx().copy_text(arn.to_string());
                            ui.close();
                        }
                    }
                });

                // Render status and age information after the tag
                if !additional_info.is_empty() {
                    ui.add_space(8.0);
                    ui.label(additional_info.join(" "));
                }

                // Render tag badges
                ui.add_space(8.0);
                self.render_tag_badges(ui, resource);

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
                        // Add View Logs and View Events buttons horizontally
                        ui.horizontal(|ui| {
                            // Add "View Logs" button if resource has associated CloudWatch Logs
                            if has_cloudwatch_logs(&resource.resource_type) {
                                if let Some(log_group) = get_log_group_name(
                                    &resource.resource_type,
                                    &resource.display_name,
                                    Some(&resource.resource_id),
                                ) {
                                    if ui.small_button("View Logs").clicked() {
                                        // Queue action to open CloudWatch Logs window
                                        self.pending_explorer_actions.push(
                                            super::ResourceExplorerAction::OpenCloudWatchLogs {
                                                log_group_name: log_group,
                                                resource_name: resource.display_name.clone(),
                                                account_id: resource.account_id.clone(),
                                                region: resource.region.clone(),
                                            },
                                        );
                                    }
                                }
                            }

                            // Add "View Events" button for CloudTrail (all resources supported)
                            if has_cloudtrail_support(&resource.resource_type)
                                && ui.small_button("View Events").clicked() {
                                    // Extract ARN from properties if available (Lambda, EC2, etc.)
                                    // Otherwise build it from resource metadata
                                    let resource_arn = extract_or_build_arn(
                                        &resource.resource_type,
                                        &resource.raw_properties,
                                        &resource.region,
                                        &resource.account_id,
                                        &resource.resource_id,
                                    );

                                    // CloudTrail ResourceName filter expects resource ID (not ARN)
                                    // Use resource_id which contains the actual resource name/identifier
                                    let filter_name = resource.resource_id.clone();

                                    log::info!(
                                        "CloudTrail: View Events clicked - resource_name='{}', filter_name='{}', resource_arn='{}', resource_type='{}'",
                                        resource.display_name,
                                        filter_name,
                                        resource_arn,
                                        resource.resource_type
                                    );

                                    // Queue action to open CloudTrail Events window
                                    self.pending_explorer_actions.push(
                                        super::ResourceExplorerAction::OpenCloudTrailEvents {
                                            resource_type: resource.resource_type.clone(),
                                            resource_name: filter_name,  // Use resource_id for filtering
                                            resource_arn: Some(resource_arn),  // Keep ARN for display
                                            account_id: resource.account_id.clone(),
                                            region: resource.region.clone(),
                                        },
                                    );
                                }
                        });
                        self.render_json_tree(ui, resource);
                    });
                });
            }
        });
    }

    /// Render JSON tree viewer for detailed resource properties
    fn render_json_tree(&mut self, ui: &mut Ui, resource: &ResourceEntry) {
        use egui_json_tree::{DefaultExpand, JsonTree};

        let resource_id = &resource.resource_id;
        let resource_id_owned = resource_id.clone();
        let resize_id = format!("json_resize_{}", resource_id);

        // Get current expand level and search term for this resource
        let current_level = self.get_expand_level(resource_id);
        let search_term = self.get_search_term(resource_id);

        // Track if we need to reset expansion state
        let mut should_reset = false;

        // JSON Tree Toolbar
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;

            // Expand All button
            if ui.small_button("Expand All").clicked() {
                self.json_expand_levels
                    .insert(resource_id_owned.clone(), 99);
                self.json_search_terms.remove(&resource_id_owned);
                should_reset = true;
            }

            // Collapse All button
            if ui.small_button("Collapse").clicked() {
                self.json_expand_levels.insert(resource_id_owned.clone(), 0);
                self.json_search_terms.remove(&resource_id_owned);
                should_reset = true;
            }

            ui.separator();

            // Level controls: - [level] +
            if ui
                .add_enabled(current_level > 0, egui::Button::new("-").small())
                .clicked()
            {
                self.json_expand_levels
                    .insert(resource_id_owned.clone(), current_level.saturating_sub(1));
                self.json_search_terms.remove(&resource_id_owned);
                should_reset = true;
            }

            ui.label(format!("L{}", current_level));

            if ui
                .add_enabled(current_level < 20, egui::Button::new("+").small())
                .clicked()
            {
                self.json_expand_levels
                    .insert(resource_id_owned.clone(), current_level.saturating_add(1));
                self.json_search_terms.remove(&resource_id_owned);
                should_reset = true;
            }

            ui.separator();

            // Search input
            let search_id = format!("json_search_{}", resource_id);
            let mut search_input = search_term.clone();
            let search_response = ui.add(
                egui::TextEdit::singleline(&mut search_input)
                    .id_salt(&search_id)
                    .hint_text("Search...")
                    .desired_width(120.0),
            );

            if search_response.changed() {
                if search_input.is_empty() {
                    self.json_search_terms.remove(&resource_id_owned);
                } else {
                    self.json_search_terms
                        .insert(resource_id_owned.clone(), search_input.clone());
                }
                should_reset = true;
            }

            // Clear search button
            if !search_term.is_empty() && ui.small_button("x").clicked() {
                self.json_search_terms.remove(&resource_id_owned);
                should_reset = true;
            }
        });

        ui.add_space(4.0);

        // Use egui's built-in Resize widget with auto-sizing and max constraints
        egui::Resize::default()
            .id_salt(&resize_id)
            .auto_sized()
            .max_height(100.0)
            .min_height(100.0)
            .max_width(800.0)
            .min_width(300.0)
            .resizable(true)
            .with_stroke(false)
            .show(ui, |ui| {
                // Determine the expand mode based on state
                let expand_mode = if !search_term.is_empty() {
                    DefaultExpand::SearchResults(&search_term)
                } else if current_level >= 99 {
                    DefaultExpand::All
                } else if current_level == 0 {
                    DefaultExpand::None
                } else {
                    DefaultExpand::ToLevel(current_level)
                };

                // JSON Tree viewer - direct rendering without wrappers
                if resource.detailed_properties.is_some() {
                    let json_data = resource.get_display_properties();
                    ui.scope(|ui| {
                        ui.style_mut()
                            .text_styles
                            .get_mut(&egui::TextStyle::Monospace)
                            .unwrap()
                            .size = 10.3;

                        let response = JsonTree::new(
                            format!("resource_json_detailed_{}", resource_id),
                            json_data,
                        )
                        .default_expand(expand_mode)
                        .show(ui);

                        if should_reset {
                            response.reset_expanded(ui);
                        }
                    });
                } else {
                    let resource_key = format!(
                        "{}:{}:{}",
                        resource.account_id, resource.region, resource.resource_id
                    );
                    if self.failed_detail_requests.contains(&resource_key) {
                        ui.horizontal(|ui| {
                            ui.colored_label(Color32::from_rgb(255, 165, 0), "!");
                            ui.label("Detailed properties not available for this resource type");
                        });
                        let json_data = resource.get_display_properties();
                        ui.scope(|ui| {
                            ui.style_mut()
                                .text_styles
                                .get_mut(&egui::TextStyle::Monospace)
                                .unwrap()
                                .size = 10.3;

                            let response = JsonTree::new(
                                format!("resource_json_basic_{}", resource_id),
                                json_data,
                            )
                            .default_expand(expand_mode)
                            .show(ui);

                            if should_reset {
                                response.reset_expanded(ui);
                            }
                        });
                    } else if self.pending_detail_requests.contains(&resource_key) {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Loading detailed properties...");
                        });
                    } else {
                        // Check if Phase 2 is loading details for this enrichable resource type
                        let enrichable_types = super::state::ResourceExplorerState::enrichable_resource_types();
                        let is_enrichable = enrichable_types.contains(&resource.resource_type.as_str());

                        if self.phase2_in_progress && is_enrichable {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(
                                    egui::RichText::new("Loading details...")
                                        .color(Color32::GRAY)
                                        .italics(),
                                );
                            });
                        }

                        let json_data = resource.get_display_properties();
                        ui.scope(|ui| {
                            ui.style_mut()
                                .text_styles
                                .get_mut(&egui::TextStyle::Monospace)
                                .unwrap()
                                .size = 10.3;

                            let response = JsonTree::new(
                                format!("resource_json_fallback_{}", resource_id),
                                json_data,
                            )
                            .default_expand(expand_mode)
                            .show(ui);

                            if should_reset {
                                response.reset_expanded(ui);
                            }
                        });
                    }
                }

                // Footer with actions
                ui.horizontal(|ui| {
                    if ui.small_button("Copy JSON").clicked() {
                        let formatted_json =
                            serde_json::to_string_pretty(resource.get_display_properties())
                                .unwrap_or_else(|_| "Error formatting JSON".to_string());
                        ui.ctx().copy_text(formatted_json);
                    }
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

    /// Maximum characters to display before truncating resource names
    const MAX_RESOURCE_NAME_CHARS: usize = 35;

    /// Render a resource type tag with colored background based on CF resource type
    ///
    /// The background color is determined by the resource_type (e.g., "AWS::EC2::Instance")
    /// so all resources of the same type have the same color, but the interior displays
    /// the resource name or ID.
    ///
    /// Returns the Response for click/context menu handling by caller.
    fn render_resource_type_tag(
        &self,
        ui: &mut Ui,
        resource_display: &str,
        resource_type: &str,
        is_expanded: bool,
    ) -> egui::Response {
        // Generate color based on resource type
        let bg_color = super::colors::assign_resource_type_color(resource_type);
        let text_color = get_contrasting_text_color(bg_color);

        // Truncate display text if not expanded and exceeds max length
        let (display_text, is_truncated) =
            if !is_expanded && resource_display.len() > Self::MAX_RESOURCE_NAME_CHARS {
                (
                    format!("{}...", &resource_display[..Self::MAX_RESOURCE_NAME_CHARS]),
                    true,
                )
            } else {
                (resource_display.to_string(), false)
            };

        // Calculate text size
        let font_size = 11.0; // Slightly larger than account/region tags
        let text_galley = ui.fonts(|fonts| {
            fonts.layout_no_wrap(
                display_text,
                egui::FontId::proportional(font_size),
                text_color,
            )
        });

        // Add padding to the text size
        let padding = egui::vec2(8.0, 3.0);
        let desired_size = text_galley.size() + 2.0 * padding;

        // Allocate space for the tag - use click sense for interaction
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            // Draw rounded rectangle background
            ui.painter().rect_filled(
                rect, 3.0, // corner radius
                bg_color,
            );

            // Draw text centered in the rect
            let text_pos = rect.center() - text_galley.size() / 2.0;
            ui.painter().galley(text_pos, text_galley, text_color);
        }

        // Show tooltip with full name and instructions
        let tooltip = if is_truncated {
            format!(
                "{}\n\nResource Type: {}\nClick to expand | Right-click to copy",
                resource_display, resource_type
            )
        } else {
            format!(
                "Resource Type: {}\nClick to collapse | Right-click to copy",
                resource_type
            )
        };
        response.clone().on_hover_text(tooltip);

        response
    }

    /// Convert AWS resource type to a short, readable tag (max 12 chars, LISP-style with hyphens)
    fn resource_type_to_short_tag(resource_type: &str) -> &'static str {
        match resource_type {
            // EC2 / Compute
            "AWS::EC2::Instance" => "INSTANCE",
            "AWS::EC2::SecurityGroup" => "SEC-GROUP",
            "AWS::EC2::VPC" => "VPC",
            "AWS::EC2::Subnet" => "SUBNET",
            "AWS::EC2::Volume" => "VOLUME",
            "AWS::EC2::Snapshot" => "SNAPSHOT",
            "AWS::EC2::Image" => "AMI",
            "AWS::EC2::InternetGateway" => "INET-GATEWAY",
            "AWS::EC2::NatGateway" => "NAT-GATEWAY",
            "AWS::EC2::RouteTable" => "ROUTE-TABLE",
            "AWS::EC2::NetworkAcl" => "NETWORK-ACL",
            "AWS::EC2::KeyPair" => "KEY-PAIR",
            "AWS::EC2::TransitGateway" => "TRANSIT-GW",
            "AWS::EC2::VPCPeeringConnection" => "VPC-PEERING",
            "AWS::EC2::FlowLog" => "FLOW-LOG",
            "AWS::EC2::VolumeAttachment" => "VOL-ATTACH",
            "AWS::EC2::NetworkInterface" => "ENI",

            // Containers
            "AWS::ECS::Cluster" => "ECS-CLUSTER",
            "AWS::ECS::Service" => "ECS-SERVICE",
            "AWS::ECS::Task" => "ECS-TASK",
            "AWS::ECS::TaskDefinition" => "TASK-DEF",
            "AWS::ECS::FargateService" => "FARGATE-SVC",
            "AWS::ECS::FargateTask" => "FARGATE-TASK",
            "AWS::EKS::Cluster" => "EKS-CLUSTER",
            "AWS::EKS::FargateProfile" => "FARGATE-PROF",
            "AWS::ECR::Repository" => "ECR-REPO",

            // Serverless
            "AWS::Lambda::Function" => "LAMBDA",
            "AWS::StepFunctions::StateMachine" => "STEP-FUNC",

            // Databases
            "AWS::RDS::DBInstance" => "RDS",
            "AWS::RDS::DBCluster" => "RDS-CLUSTER",
            "AWS::DynamoDB::Table" => "DYNAMODB",
            "AWS::Redshift::Cluster" => "REDSHIFT",
            "AWS::Neptune::DBCluster" => "NEPTUNE",
            "AWS::Neptune::DBInstance" => "NEPTUNE-INST",
            "AWS::DocumentDB::Cluster" => "DOCUMENTDB",
            "AWS::ElastiCache::CacheCluster" => "ELASTICACHE",
            "AWS::ElastiCache::ReplicationGroup" => "CACHE-REPLGR",

            // Storage
            "AWS::S3::Bucket" => "S3",
            "AWS::EFS::FileSystem" => "EFS",
            "AWS::FSx::FileSystem" => "FSX",
            "AWS::FSx::Backup" => "FSX-BACKUP",
            "AWS::Backup::BackupPlan" => "BACKUP-PLAN",
            "AWS::Backup::BackupVault" => "BACKUP-VAULT",

            // Networking
            "AWS::Route53::HostedZone" => "ROUTE53",
            "AWS::ElasticLoadBalancing::LoadBalancer" => "ELB-CLASSIC",
            "AWS::ElasticLoadBalancingV2::LoadBalancer" => "ALB",
            "AWS::ElasticLoadBalancingV2::TargetGroup" => "TARGET-GROUP",
            "AWS::ApiGateway::RestApi" => "API-GATEWAY",
            "AWS::ApiGatewayV2::Api" => "APIGW-HTTP",
            "AWS::CloudFront::Distribution" => "CLOUDFRONT",
            "AWS::GlobalAccelerator::Accelerator" => "GLOBAL-ACCEL",

            // Messaging
            "AWS::SQS::Queue" => "SQS",
            "AWS::SNS::Topic" => "SNS",
            "AWS::Events::EventBus" => "EVENT-BUS",
            "AWS::Events::Rule" => "EVENT-RULE",
            "AWS::Kinesis::Stream" => "KINESIS",
            "AWS::KinesisFirehose::DeliveryStream" => "FIREHOSE",
            "AWS::MSK::Cluster" => "MSK",
            "AWS::AmazonMQ::Broker" => "AMAZON-MQ",

            // Security & Identity
            "AWS::IAM::Role" => "IAM-ROLE",
            "AWS::IAM::User" => "IAM-USER",
            "AWS::IAM::Group" => "IAM-GROUP",
            "AWS::IAM::Policy" => "IAM-POLICY",
            "AWS::KMS::Key" => "KMS",
            "AWS::SecretsManager::Secret" => "SECRET",
            "AWS::AccessAnalyzer::Analyzer" => "ACCESS-ANLZR",
            "AWS::GuardDuty::Detector" => "GUARDDUTY",
            "AWS::SecurityHub::Hub" => "SECURITYHUB",
            "AWS::Shield::Protection" => "SHIELD",
            "AWS::Shield::Subscription" => "SHIELD-SUB",
            "AWS::Detective::Graph" => "DETECTIVE",
            "AWS::Inspector::Configuration" => "INSPECTOR",
            "AWS::ACM::Certificate" => "ACM",
            "AWS::ACMPCA::CertificateAuthority" => "ACM-PCA",
            "AWS::WAFv2::WebACL" => "WAF",

            // Management & Monitoring
            "AWS::CloudWatch::Alarm" => "CW-ALARM",
            "AWS::CloudWatch::Dashboard" => "CW-DASH",
            "AWS::Logs::LogGroup" => "LOG-GROUP",
            "AWS::Config::ConfigRule" => "CONFIG-RULE",
            "AWS::Config::ConfigurationRecorder" => "CONFIG-REC",
            "AWS::CloudTrail::Trail" => "CLOUDTRAIL",
            "AWS::XRay::SamplingRule" => "XRAY",
            "AWS::CloudFormation::Stack" => "CFN-STACK",

            // AI/ML
            "AWS::Bedrock::Model" => "BEDROCK",
            "AWS::SageMaker::Endpoint" => "SAGEMAKER",
            "AWS::SageMaker::Model" => "SAGE-MODEL",
            "AWS::SageMaker::TrainingJob" => "SAGE-TRAIN",
            "AWS::Rekognition::Collection" => "REKOGNITION",
            "AWS::Rekognition::StreamProcessor" => "REKOG-STRM",
            "AWS::Lex::Bot" => "LEX",
            "AWS::Polly::Voice" => "POLLY",

            // Application Integration
            "AWS::AppSync::GraphQLApi" => "APPSYNC",
            "AWS::Cognito::UserPool" => "USER-POOL",
            "AWS::Cognito::IdentityPool" => "ID-POOL",

            // Developer Tools
            "AWS::CodeCommit::Repository" => "CODECOMMIT",
            "AWS::CodePipeline::Pipeline" => "CODEPIPELINE",
            "AWS::CodeBuild::Project" => "CODEBUILD",

            // Analytics
            "AWS::Glue::Database" => "GLUE-DB",
            "AWS::Glue::Table" => "GLUE-TABLE",
            "AWS::Glue::Crawler" => "GLUE-CRAWLR",
            "AWS::Athena::WorkGroup" => "ATHENA",
            "AWS::QuickSight::DataSource" => "QUICKSIGHT",
            "AWS::Timestream::Database" => "TIMESTREAM",
            "AWS::OpenSearchService::Domain" => "OPENSEARCH",
            "AWS::CloudWatch::CompositeAlarm" => "CW-COMP",
            "AWS::CloudWatch::Metric" => "CW-METRIC",
            "AWS::CloudWatch::InsightRule" => "CW-INSIGHT",
            "AWS::CloudWatch::AnomalyDetector" => "CW-ANOM",
            "AWS::Logs::LogStream" => "LOG-STREAM",
            "AWS::Logs::MetricFilter" => "LOG-METRIC",
            "AWS::Logs::SubscriptionFilter" => "LOG-SUB",
            "AWS::Logs::ResourcePolicy" => "LOG-POLICY",
            "AWS::Logs::QueryDefinition" => "LOG-QUERY",

            // Organizations
            "AWS::Organizations::Organization" => "ORG",
            "AWS::Organizations::Account" => "ORG-ACCOUNT",
            "AWS::Organizations::OrganizationalUnit" => "ORG-UNIT",
            "AWS::Organizations::Policy" => "ORG-POLICY",
            "AWS::Organizations::Root" => "ORG-ROOT",
            "AWS::Organizations::AwsServiceAccess" => "ORG-SVC-ACC",
            "AWS::Organizations::Handshake" => "ORG-HANDSHK",
            "AWS::Organizations::DelegatedAdmin" => "ORG-DELEG",
            "AWS::Organizations::CreateAccountStatus" => "ORG-ACCT-ST",

            // IoT
            "AWS::IoT::Thing" => "IOT-THING",
            "AWS::GreengrassV2::ComponentVersion" => "GREENGRASS",

            // Other Services
            "AWS::DataBrew::Job" => "DATABREW",
            "AWS::DataBrew::Dataset" => "DATABREW-DS",
            "AWS::DataSync::Task" => "DATASYNC",
            "AWS::DataSync::Location" => "DSYNC-LOC",
            "AWS::WorkSpaces::Workspace" => "WORKSPACE",
            "AWS::WorkSpaces::Directory" => "WKSP-DIR",
            "AWS::Transfer::Server" => "TRANSFER",
            "AWS::Transfer::User" => "XFER-USER",
            "AWS::Connect::Instance" => "CONNECT",
            "AWS::Amplify::App" => "AMPLIFY",
            "AWS::Macie::Session" => "MACIE",
            "AWS::Batch::JobQueue" => "BATCH",
            "AWS::EMR::Cluster" => "EMR",
            "AWS::LakeFormation::Resource" => "LAKEFORM",
            "AWS::EC2::ElasticIP" => "EIP",
            "AWS::EC2::LaunchTemplate" => "LT",
            "AWS::EC2::PlacementGroup" => "PLACEMENT",
            "AWS::EC2::ReservedInstance" => "RI",
            "AWS::EC2::SpotInstanceRequest" => "SPOT",
            "AWS::EC2::DHCPOptions" => "DHCP",
            "AWS::EC2::EgressOnlyInternetGateway" => "EIGW",
            "AWS::EC2::VPNConnection" => "VPN-CONN",
            "AWS::EC2::VPNGateway" => "VPN-GW",
            "AWS::EC2::CustomerGateway" => "CUST-GW",
            "AWS::ECS::CapacityProvider" => "ECS-CAP",
            "AWS::ECS::TaskSet" => "ECS-TS",
            "AWS::EKS::IdentityProviderConfig" => "EKS-IDP",
            "AWS::IAM::ServerCertificate" => "IAM-CERT",
            "AWS::RDS::DBClusterSnapshot" => "RDS-CLSNAP",
            "AWS::RDS::OptionGroup" => "RDS-OPT",

            // Fallback: extract service name from resource type
            _ => {
                // Try to extract a reasonable short name from unknown types
                // AWS::Service::ResourceType -> SERVICE
                if let Some(service) = resource_type.split("::").nth(1) {
                    // Return a static str for common services, otherwise use leaked string
                    match service {
                        "EC2" => "EC2",
                        "S3" => "S3",
                        "Lambda" => "LAMBDA",
                        "IAM" => "IAM",
                        "RDS" => "RDS",
                        "DynamoDB" => "DYNAMODB",
                        "SQS" => "SQS",
                        "SNS" => "SNS",
                        "ECS" => "ECS",
                        "EKS" => "EKS",
                        _ => {
                            let upper = service
                                .chars()
                                .filter(|c| c.is_ascii_alphanumeric())
                                .collect::<String>()
                                .to_ascii_uppercase();
                            Box::leak(upper.into_boxed_str())
                        }
                    }
                } else {
                    "AWS"
                }
            }
        }
    }

    /// Render a short resource type tag (e.g., "LAMBDA", "EC2-INSTANCE")
    fn render_resource_type_short_tag(&self, ui: &mut Ui, resource_type: &str) {
        let short_tag = Self::resource_type_to_short_tag(resource_type);

        // Use the same color as the resource type for consistency
        let bg_color = super::colors::assign_resource_type_color(resource_type);
        let text_color = get_contrasting_text_color(bg_color);

        // Calculate text size - use monospace for consistent width
        let font_size = 9.0;
        let text_galley = ui.fonts(|fonts| {
            fonts.layout_no_wrap(
                short_tag.to_string(),
                egui::FontId::monospace(font_size),
                text_color,
            )
        });

        // Add padding to the text size
        let padding = egui::vec2(4.0, 2.0);
        let desired_size = text_galley.size() + 2.0 * padding;

        // Allocate space for the tag
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            // Draw rounded rectangle background
            ui.painter().rect_filled(
                rect, 2.0, // corner radius
                bg_color,
            );

            // Draw text centered in the rect
            let text_pos = rect.center() - text_galley.size() / 2.0;
            ui.painter().galley(text_pos, text_galley, text_color);
        }

        // Show tooltip with full resource type on hover
        response.on_hover_text(format!("Resource Type: {}", resource_type));
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

    /// Render tag badges for a resource based on popularity and filters
    fn render_tag_badges(&mut self, ui: &mut Ui, resource: &super::state::ResourceEntry) {
        // Only render if we have badge selector and tag popularity
        if let (Some(badge_selector), Some(tag_popularity)) =
            (&self.badge_selector, &self.tag_popularity)
        {
            // Get badges to display for this resource
            let badges = badge_selector.select_badges(resource, tag_popularity, 10);

            // Render each badge
            for badge in badges {
                ui.add_space(4.0); // Space between badges

                // Generate color based on tag key for visual consistency
                let color_generator = super::colors::AwsColorGenerator::new();
                let badge_color = color_generator.get_tag_key_color(&badge.key);
                let text_color = super::colors::get_contrasting_text_color(badge_color);

                // Format badge text (truncate if too long)
                let badge_text = badge.short_display(20);

                // Calculate text size
                let font_size = 9.0;
                let text_galley = ui.fonts(|fonts| {
                    fonts.layout_no_wrap(
                        badge_text.clone(),
                        egui::FontId::monospace(font_size),
                        text_color,
                    )
                });

                // Add padding to the text size
                let padding = egui::vec2(4.0, 1.0);
                let desired_size = text_galley.size() + 2.0 * padding;

                // Allocate space for the badge with click sensing
                let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

                if ui.is_rect_visible(rect) {
                    // Determine if badge is hovered for visual feedback
                    let is_hovered = response.hovered();

                    // Lighten color on hover for interactivity feedback
                    let final_color = if is_hovered {
                        // Lighten the badge color slightly on hover
                        let r = (badge_color.r() as f32 * 1.15).min(255.0) as u8;
                        let g = (badge_color.g() as f32 * 1.15).min(255.0) as u8;
                        let b = (badge_color.b() as f32 * 1.15).min(255.0) as u8;
                        Color32::from_rgb(r, g, b)
                    } else {
                        badge_color
                    };

                    // Draw rounded rectangle background
                    ui.painter().rect_filled(
                        rect,
                        2.0, // corner radius (slightly smaller than region tags)
                        final_color,
                    );

                    // Draw text centered in the rect
                    let text_pos = rect.center() - text_galley.size() / 2.0;
                    ui.painter().galley(text_pos, text_galley, text_color);
                }

                // Handle click (check before hover to avoid borrow issues)
                let was_clicked = response.clicked();

                // Show enhanced tooltip with tag information and count
                let tag_count = tag_popularity.get_count(&badge);
                let tag_percentage = tag_popularity.get_percentage(&badge);
                response.on_hover_text(format!(
                    "{}\n\n{} resources ({:.1}%)\n\nClick to add this tag to filters",
                    badge.display_name(),
                    tag_count,
                    tag_percentage
                ));

                // Handle click - add to pending actions for processing
                if was_clicked {
                    tracing::info!(
                        "Tag badge clicked: {} = {} (adding to filter)",
                        badge.key,
                        badge.value
                    );

                    self.pending_tag_clicks.push(super::state::TagClickAction {
                        tag_key: badge.key.clone(),
                        tag_value: badge.value.clone(),
                    });
                }
            }
        }
    }
}

/// Extract ARN from resource properties or build it from metadata
///
/// Tries to extract ARN from raw_properties first (Lambda has FunctionArn, EC2 has Arn, etc.),
/// then falls back to constructing it based on AWS resource type patterns.
///
/// Note: This ARN is for display purposes. CloudTrail filtering uses resource_id (resource name), not ARN.
fn extract_or_build_arn(
    resource_type: &str,
    raw_properties: &serde_json::Value,
    region: &str,
    account_id: &str,
    resource_id: &str,
) -> String {
    // Try to extract ARN from properties first (only if it's an object)
    if let Some(props_map) = raw_properties.as_object() {
        // Different AWS services store ARN in different property names
        let arn_property_names = match resource_type {
            "AWS::Lambda::Function" => vec!["FunctionArn", "Arn"],
            "AWS::EC2::Instance" => vec!["Arn", "InstanceArn"],
            "AWS::DynamoDB::Table" => vec!["TableArn", "Arn"],
            "AWS::IAM::Role" => vec!["Arn", "RoleArn"],
            "AWS::IAM::User" => vec!["Arn", "UserArn"],
            "AWS::S3::Bucket" => vec!["Arn", "BucketArn"],
            _ => vec!["Arn"],
        };

        // Try each property name
        for property_name in arn_property_names {
            if let Some(arn_value) = props_map.get(property_name) {
                if let Some(arn_str) = arn_value.as_str() {
                    log::debug!(
                        "CloudTrail: Extracted ARN from property '{}' for resource_type='{}'",
                        property_name,
                        resource_type
                    );
                    return arn_str.to_string();
                }
            }
        }
    }

    // Fall back to building ARN if not found in properties
    log::debug!(
        "CloudTrail: ARN not found in properties for resource_type='{}', building it",
        resource_type
    );

    match resource_type {
        // Lambda functions
        "AWS::Lambda::Function" => {
            format!(
                "arn:aws:lambda:{}:{}:function:{}",
                region, account_id, resource_id
            )
        }
        // EC2 Instances
        "AWS::EC2::Instance" => {
            format!(
                "arn:aws:ec2:{}:{}:instance/{}",
                region, account_id, resource_id
            )
        }
        // S3 Buckets (buckets are global, no region in ARN)
        "AWS::S3::Bucket" => {
            format!("arn:aws:s3:::{}", resource_id)
        }
        // DynamoDB Tables
        "AWS::DynamoDB::Table" => {
            format!(
                "arn:aws:dynamodb:{}:{}:table/{}",
                region, account_id, resource_id
            )
        }
        // IAM Roles (IAM is global, no region)
        "AWS::IAM::Role" => {
            format!("arn:aws:iam::{}:role/{}", account_id, resource_id)
        }
        // IAM Users (IAM is global, no region)
        "AWS::IAM::User" => {
            format!("arn:aws:iam::{}:user/{}", account_id, resource_id)
        }
        // Default fallback - just return the resource_id
        _ => resource_id.to_string(),
    }
}
