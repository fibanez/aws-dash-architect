use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceEntry {
    pub resource_type: String, // AWS::EC2::Instance
    pub account_id: String,
    pub region: String,
    pub resource_id: String,
    pub display_name: String, // Human readable name (instance name, role name, etc.)
    pub status: Option<String>, // Instance state, enabled/disabled, etc.
    pub properties: serde_json::Value, // Normalized AWS API response
    pub raw_properties: serde_json::Value, // Original AWS API response (from List queries)
    pub detailed_properties: Option<serde_json::Value>, // Detailed properties from Describe queries
    pub detailed_timestamp: Option<DateTime<Utc>>, // When detailed data was fetched
    pub tags: Vec<ResourceTag>,
    pub relationships: Vec<ResourceRelationship>, // Connections to other resources
    pub parent_resource_id: Option<String>,       // ID of parent resource (for child resources)
    pub parent_resource_type: Option<String>,     // Type of parent resource (for child resources)
    pub is_child_resource: bool,                  // Flag to identify child resources
    pub account_color: egui::Color32,             // Consistent color for account
    pub region_color: egui::Color32,              // Consistent color for region
    pub query_timestamp: DateTime<Utc>,           // When this resource data was fetched
}

impl ResourceEntry {
    /// Estimate the memory footprint of this resource entry in bytes
    ///
    /// This provides an approximate calculation of heap-allocated memory
    /// including strings, JSON values, and vectors.
    pub fn estimate_memory_bytes(&self) -> usize {
        let mut size = std::mem::size_of::<Self>(); // Stack size

        // String fields
        size += self.resource_type.capacity();
        size += self.account_id.capacity();
        size += self.region.capacity();
        size += self.resource_id.capacity();
        size += self.display_name.capacity();
        if let Some(ref status) = self.status {
            size += status.capacity();
        }
        if let Some(ref parent_id) = self.parent_resource_id {
            size += parent_id.capacity();
        }
        if let Some(ref parent_type) = self.parent_resource_type {
            size += parent_type.capacity();
        }

        // JSON values (approximate based on serialized size)
        size += self.properties.to_string().len();
        size += self.raw_properties.to_string().len();
        if let Some(ref detailed) = self.detailed_properties {
            size += detailed.to_string().len();
        }

        // Tags vector
        size += self.tags.capacity() * std::mem::size_of::<ResourceTag>();
        for tag in &self.tags {
            size += tag.key.capacity() + tag.value.capacity();
        }

        // Relationships vector
        size += self.relationships.capacity() * std::mem::size_of::<ResourceRelationship>();
        for rel in &self.relationships {
            size += rel.target_resource_id.capacity();
            size += rel.target_resource_type.capacity();
        }

        size
    }

    /// Check if this resource data is considered stale (older than threshold)
    pub fn is_stale(&self, stale_threshold_minutes: i64) -> bool {
        let now = Utc::now();
        let age = now.signed_duration_since(self.query_timestamp);
        age.num_minutes() > stale_threshold_minutes
    }

    /// Get a human-readable age string for the resource data
    pub fn get_age_display(&self) -> String {
        let now = Utc::now();
        let age = now.signed_duration_since(self.query_timestamp);

        if age.num_days() > 0 {
            format!("{}d ago", age.num_days())
        } else if age.num_hours() > 0 {
            format!("{}h ago", age.num_hours())
        } else if age.num_minutes() > 0 {
            format!("{}m ago", age.num_minutes())
        } else {
            "Just now".to_string()
        }
    }

    /// Check if detailed properties are available and fresh
    pub fn has_fresh_detailed_properties(&self, stale_threshold_minutes: i64) -> bool {
        if let (Some(_), Some(timestamp)) = (&self.detailed_properties, &self.detailed_timestamp) {
            let now = Utc::now();
            let age = now.signed_duration_since(*timestamp);
            age.num_minutes() <= stale_threshold_minutes
        } else {
            false
        }
    }

    /// Set detailed properties with current timestamp
    pub fn set_detailed_properties(&mut self, properties: serde_json::Value) {
        self.detailed_properties = Some(properties);
        self.detailed_timestamp = Some(Utc::now());
    }

    /// Get the best available properties (detailed if available, otherwise raw)
    pub fn get_display_properties(&self) -> &serde_json::Value {
        self.detailed_properties
            .as_ref()
            .unwrap_or(&self.raw_properties)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceTag {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRelationship {
    pub relationship_type: RelationshipType,
    pub target_resource_id: String,
    pub target_resource_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipType {
    Uses,            // EC2 uses Security Group
    Contains,        // VPC contains Subnet
    ChildOf,         // DataSource is child of KnowledgeBase
    ParentOf,        // KnowledgeBase is parent of DataSource
    AttachedTo,      // EBS attached to EC2
    MemberOf,        // User member of Group
    DeployedIn,      // MQ Broker deployed in Subnet
    ProtectedBy,     // MQ Broker protected by Security Group
    DeadLetterQueue, // SQS Queue uses another queue as DLQ
    ServesAsDlq,     // SQS Queue serves as DLQ for another queue
}

// ============================================================================
// Tag Filtering Data Structures
// ============================================================================

/// Type of tag filter operation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TagFilterType {
    /// Tag key equals specific value(s)
    Equals,
    /// Tag key does not equal specific value(s)
    NotEquals,
    /// Tag key contains substring
    Contains,
    /// Tag key does not contain substring
    NotContains,
    /// Tag key starts with prefix
    StartsWith,
    /// Tag key ends with suffix
    EndsWith,
    /// Tag key matches regex pattern
    Regex,
    /// Tag key exists on resource (any value)
    Exists,
    /// Tag key does not exist on resource
    NotExists,
    /// Tag value is in list of values
    In,
    /// Tag value is not in list of values
    NotIn,
}

impl TagFilterType {
    pub fn display_name(&self) -> &'static str {
        match self {
            TagFilterType::Equals => "Equals",
            TagFilterType::NotEquals => "Not Equals",
            TagFilterType::Contains => "Contains",
            TagFilterType::NotContains => "Not Contains",
            TagFilterType::StartsWith => "Starts With",
            TagFilterType::EndsWith => "Ends With",
            TagFilterType::Regex => "Regex",
            TagFilterType::Exists => "Exists",
            TagFilterType::NotExists => "Not Exists",
            TagFilterType::In => "In",
            TagFilterType::NotIn => "Not In",
        }
    }

    pub fn all_types() -> Vec<TagFilterType> {
        vec![
            TagFilterType::Equals,
            TagFilterType::NotEquals,
            TagFilterType::Contains,
            TagFilterType::NotContains,
            TagFilterType::StartsWith,
            TagFilterType::EndsWith,
            TagFilterType::Regex,
            TagFilterType::Exists,
            TagFilterType::NotExists,
            TagFilterType::In,
            TagFilterType::NotIn,
        ]
    }

    /// Check if this filter type requires value(s)
    pub fn requires_values(&self) -> bool {
        !matches!(self, TagFilterType::Exists | TagFilterType::NotExists)
    }

    /// Check if this filter type supports multiple values
    pub fn supports_multiple_values(&self) -> bool {
        matches!(
            self,
            TagFilterType::In | TagFilterType::NotIn | TagFilterType::Equals | TagFilterType::NotEquals
        )
    }
}

/// Boolean operator for combining filters
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BooleanOperator {
    And,
    Or,
}

impl BooleanOperator {
    pub fn display_name(&self) -> &'static str {
        match self {
            BooleanOperator::And => "AND",
            BooleanOperator::Or => "OR",
        }
    }

    pub fn all_operators() -> Vec<BooleanOperator> {
        vec![BooleanOperator::And, BooleanOperator::Or]
    }
}

/// Represents a tag badge click action for filtering
#[derive(Debug, Clone)]
pub struct TagClickAction {
    pub tag_key: String,
    pub tag_value: String,
}

/// A single tag filter condition
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TagFilter {
    /// Tag key to filter on (e.g., "Environment", "Team")
    pub tag_key: String,
    /// Type of filter operation
    pub filter_type: TagFilterType,
    /// Values to compare against (for Equals, In, etc.)
    /// Empty for Exists/NotExists filters
    pub values: Vec<String>,
    /// Regex pattern (for Regex filter type)
    pub pattern: Option<String>,
}

impl TagFilter {
    pub fn new(tag_key: String, filter_type: TagFilterType) -> Self {
        Self {
            tag_key,
            filter_type,
            values: Vec::new(),
            pattern: None,
        }
    }

    pub fn with_values(mut self, values: Vec<String>) -> Self {
        self.values = values;
        self
    }

    pub fn with_pattern(mut self, pattern: String) -> Self {
        self.pattern = Some(pattern);
        self
    }

    /// Validate that this filter is properly configured
    pub fn is_valid(&self) -> bool {
        // Tag key must not be empty
        if self.tag_key.is_empty() {
            return false;
        }

        // Check value requirements based on filter type
        match self.filter_type {
            TagFilterType::Exists | TagFilterType::NotExists => {
                // These don't require values
                true
            }
            TagFilterType::Regex => {
                // Regex requires a pattern
                self.pattern.is_some() && !self.pattern.as_ref().unwrap().is_empty()
            }
            _ => {
                // All other types require at least one value
                !self.values.is_empty()
            }
        }
    }

    /// Evaluate this filter against a resource
    /// Returns true if the resource matches this filter
    pub fn matches(&self, resource: &ResourceEntry) -> bool {
        // Find the tag value for this key on the resource
        let tag_value = resource.tags.iter()
            .find(|tag| tag.key == self.tag_key)
            .map(|tag| tag.value.as_str());

        match self.filter_type {
            TagFilterType::Exists => {
                // Resource must have this tag key (any value)
                tag_value.is_some()
            }
            TagFilterType::NotExists => {
                // Resource must not have this tag key
                tag_value.is_none()
            }
            TagFilterType::Equals => {
                // Tag value must equal one of the specified values
                if let Some(value) = tag_value {
                    self.values.iter().any(|v| v == value)
                } else {
                    false
                }
            }
            TagFilterType::NotEquals => {
                // Tag value must not equal any of the specified values
                if let Some(value) = tag_value {
                    !self.values.iter().any(|v| v == value)
                } else {
                    // Tag doesn't exist, so it's not equal to any value
                    true
                }
            }
            TagFilterType::Contains => {
                // Tag value must contain the substring
                if let Some(value) = tag_value {
                    self.values.iter().any(|v| value.contains(v))
                } else {
                    false
                }
            }
            TagFilterType::NotContains => {
                // Tag value must not contain the substring
                if let Some(value) = tag_value {
                    !self.values.iter().any(|v| value.contains(v))
                } else {
                    // Tag doesn't exist, so it doesn't contain anything
                    true
                }
            }
            TagFilterType::StartsWith => {
                // Tag value must start with the prefix
                if let Some(value) = tag_value {
                    self.values.iter().any(|v| value.starts_with(v))
                } else {
                    false
                }
            }
            TagFilterType::EndsWith => {
                // Tag value must end with the suffix
                if let Some(value) = tag_value {
                    self.values.iter().any(|v| value.ends_with(v))
                } else {
                    false
                }
            }
            TagFilterType::In => {
                // Tag value must be in the list of values
                if let Some(value) = tag_value {
                    self.values.iter().any(|v| v == value)
                } else {
                    false
                }
            }
            TagFilterType::NotIn => {
                // Tag value must not be in the list of values
                if let Some(value) = tag_value {
                    !self.values.iter().any(|v| v == value)
                } else {
                    // Tag doesn't exist, so it's not in any list
                    true
                }
            }
            TagFilterType::Regex => {
                // Tag value must match the regex pattern
                if let Some(value) = tag_value {
                    if let Some(pattern) = &self.pattern {
                        // Try to compile and match the regex
                        // Note: In production, we should cache compiled regexes
                        if let Ok(re) = regex::Regex::new(pattern) {
                            re.is_match(value)
                        } else {
                            // Invalid regex pattern - don't match
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
        }
    }
}

/// A group of tag filters combined with a boolean operator
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TagFilterGroup {
    /// Boolean operator for combining filters (AND/OR)
    pub operator: BooleanOperator,
    /// Individual tag filters in this group
    pub filters: Vec<TagFilter>,
    /// Nested sub-groups for complex boolean logic
    pub sub_groups: Vec<TagFilterGroup>,
}

impl Default for TagFilterGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl TagFilterGroup {
    pub fn new() -> Self {
        Self {
            operator: BooleanOperator::And,
            filters: Vec::new(),
            sub_groups: Vec::new(),
        }
    }

    pub fn with_operator(mut self, operator: BooleanOperator) -> Self {
        self.operator = operator;
        self
    }

    pub fn add_filter(&mut self, filter: TagFilter) {
        self.filters.push(filter);
    }

    pub fn add_sub_group(&mut self, group: TagFilterGroup) {
        self.sub_groups.push(group);
    }

    /// Check if this group has any filters or sub-groups
    pub fn is_empty(&self) -> bool {
        self.filters.is_empty() && self.sub_groups.is_empty()
    }

    /// Count total number of filters (including nested)
    pub fn filter_count(&self) -> usize {
        let direct_filters = self.filters.len();
        let nested_filters: usize = self.sub_groups.iter().map(|g| g.filter_count()).sum();
        direct_filters + nested_filters
    }

    /// Validate that all filters in this group are valid
    pub fn is_valid(&self) -> bool {
        // Empty groups are valid (no filtering)
        if self.is_empty() {
            return true;
        }

        // Check all direct filters
        if !self.filters.iter().all(|f| f.is_valid()) {
            return false;
        }

        // Check all sub-groups recursively
        self.sub_groups.iter().all(|g| g.is_valid())
    }

    /// Collect all unique tag keys from filters (recursively)
    /// Used to identify which tags should be prioritized in badge display
    pub fn collect_filter_tag_keys(&self, tag_keys: &mut Vec<String>) {
        // Collect from direct filters
        for filter in &self.filters {
            if !tag_keys.contains(&filter.tag_key) {
                tag_keys.push(filter.tag_key.clone());
            }
        }

        // Recursively collect from sub-groups
        for sub_group in &self.sub_groups {
            sub_group.collect_filter_tag_keys(tag_keys);
        }
    }

    /// Evaluate this filter group against a resource
    /// Returns true if the resource matches the filter criteria
    pub fn matches(&self, resource: &ResourceEntry) -> bool {
        // Empty group matches everything
        if self.is_empty() {
            return true;
        }

        // Evaluate all direct filters
        let filter_results: Vec<bool> = self.filters.iter()
            .map(|filter| filter.matches(resource))
            .collect();

        // Evaluate all sub-groups recursively
        let subgroup_results: Vec<bool> = self.sub_groups.iter()
            .map(|group| group.matches(resource))
            .collect();

        // Combine all results (filters + sub-groups) based on the operator
        let all_results: Vec<bool> = filter_results.into_iter()
            .chain(subgroup_results)
            .collect();

        if all_results.is_empty() {
            // No conditions to evaluate - matches by default
            return true;
        }

        match self.operator {
            BooleanOperator::And => {
                // ALL conditions must be true
                all_results.iter().all(|&result| result)
            }
            BooleanOperator::Or => {
                // At least ONE condition must be true
                all_results.iter().any(|&result| result)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryScope {
    pub accounts: Vec<AccountSelection>,
    pub regions: Vec<RegionSelection>,
    pub resource_types: Vec<ResourceTypeSelection>,
}

impl Default for QueryScope {
    fn default() -> Self {
        Self::new()
    }
}

impl QueryScope {
    pub fn new() -> Self {
        Self {
            accounts: Vec::new(),
            regions: Vec::new(),
            resource_types: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty() || self.regions.is_empty() || self.resource_types.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountSelection {
    pub account_id: String,
    pub display_name: String,
    pub color: egui::Color32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionSelection {
    pub region_code: String,
    pub display_name: String,
    pub color: egui::Color32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceTypeSelection {
    pub resource_type: String,
    pub display_name: String,
    pub service_name: String, // EC2, IAM, Bedrock, etc.
}

#[derive(Debug)]
pub struct ResourceExplorerState {
    pub resources: Vec<ResourceEntry>,
    pub query_scope: QueryScope,
    pub search_filter: String,
    pub primary_grouping: GroupingMode,
    pub tag_filter_group: TagFilterGroup, // Tag-based filtering
    pub tag_discovery: crate::app::resource_explorer::tag_discovery::TagDiscovery, // Tag metadata and discovery
    pub tag_popularity: crate::app::resource_explorer::tag_badges::TagPopularityTracker, // Tag popularity tracking
    pub badge_selector: crate::app::resource_explorer::tag_badges::BadgeSelector, // Badge selection strategy
    pub property_catalog: crate::app::resource_explorer::PropertyCatalog, // Property discovery and metadata
    pub property_filter_group: crate::app::resource_explorer::PropertyFilterGroup, // Property-based filtering
    pub loading_tasks: HashSet<String>,   // Track active queries
    pub cached_queries: HashMap<String, Vec<ResourceEntry>>, // Session cache
    pub show_refresh_dialog: bool,
    pub show_account_dialog: bool,
    pub show_region_dialog: bool,
    pub show_resource_type_dialog: bool,
    pub stale_data_threshold_minutes: i64, // Data older than this is considered stale
    // Tag filtering UI state
    pub show_only_tagged: bool,        // Filter to only resources with tags
    pub show_only_untagged: bool,      // Filter to only resources without tags
    pub show_filter_builder: bool,     // Show advanced filter builder dialog
    pub show_property_filter_builder: bool, // Show property filter builder dialog
    // Tag grouping UI state
    pub show_tag_hierarchy_builder: bool, // Show tag hierarchy builder dialog
    pub min_tag_resources_for_grouping: usize, // Minimum resource count for tags to appear in GroupBy dropdown
    // Property grouping UI state (M6)
    pub show_property_hierarchy_builder: bool, // Show property hierarchy builder dialog
}

impl Default for ResourceExplorerState {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceExplorerState {
    pub fn new() -> Self {
        Self {
            resources: Vec::new(),
            query_scope: QueryScope::new(),
            search_filter: String::new(),
            primary_grouping: GroupingMode::ByAccount,
            tag_filter_group: TagFilterGroup::new(), // Empty filter group by default
            tag_discovery: crate::app::resource_explorer::tag_discovery::TagDiscovery::new(), // Initialize tag discovery
            tag_popularity: crate::app::resource_explorer::tag_badges::TagPopularityTracker::new(), // Initialize tag popularity tracker
            badge_selector: crate::app::resource_explorer::tag_badges::BadgeSelector::new(), // Initialize badge selector
            property_catalog: crate::app::resource_explorer::PropertyCatalog::new(), // Initialize property catalog
            property_filter_group: crate::app::resource_explorer::PropertyFilterGroup::new(), // Empty property filter group by default
            loading_tasks: HashSet::new(),
            cached_queries: HashMap::new(),
            show_refresh_dialog: false,
            show_account_dialog: false,
            show_region_dialog: false,
            show_resource_type_dialog: false,
            stale_data_threshold_minutes: 15, // Consider data stale after 15 minutes
            show_only_tagged: false,
            show_only_untagged: false,
            show_filter_builder: false,
            show_property_filter_builder: false,
            show_tag_hierarchy_builder: false,
            min_tag_resources_for_grouping: 1, // Default: show all tags with at least 1 resource
            show_property_hierarchy_builder: false,
        }
    }

    pub fn add_account(&mut self, account: AccountSelection) {
        if !self
            .query_scope
            .accounts
            .iter()
            .any(|a| a.account_id == account.account_id)
        {
            self.query_scope.accounts.push(account);
        }
    }

    pub fn remove_account(&mut self, account_id: &str) {
        self.query_scope
            .accounts
            .retain(|a| a.account_id != account_id);

        // Rebuild property catalog from resources in active selection
        self.rebuild_property_catalog_from_active_selection();
    }

    pub fn add_region(&mut self, region: RegionSelection) {
        if !self
            .query_scope
            .regions
            .iter()
            .any(|r| r.region_code == region.region_code)
        {
            self.query_scope.regions.push(region);
        }
    }

    pub fn remove_region(&mut self, region_code: &str) {
        self.query_scope
            .regions
            .retain(|r| r.region_code != region_code);

        // Rebuild property catalog from resources in active selection
        self.rebuild_property_catalog_from_active_selection();
    }

    pub fn add_resource_type(&mut self, resource_type: ResourceTypeSelection) {
        if !self
            .query_scope
            .resource_types
            .iter()
            .any(|rt| rt.resource_type == resource_type.resource_type)
        {
            self.query_scope.resource_types.push(resource_type);
        }
    }

    pub fn remove_resource_type(&mut self, resource_type: &str) {
        self.query_scope
            .resource_types
            .retain(|rt| rt.resource_type != resource_type);

        // Rebuild property catalog from resources in active selection
        self.rebuild_property_catalog_from_active_selection();
    }

    pub fn generate_cache_key(&self, account: &str, region: &str, resource_type: &str) -> String {
        format!("{}:{}:{}", account, region, resource_type)
    }

    pub fn is_loading(&self) -> bool {
        !self.loading_tasks.is_empty()
    }

    /// Add a loading task and return the key for cleanup
    pub fn start_loading_task(&mut self, task_name: &str) -> String {
        let cache_key = format!("{}_{}", task_name, chrono::Utc::now().timestamp_millis());
        self.loading_tasks.insert(cache_key.clone());
        tracing::info!(
            "🔄 Started loading task: {} (total: {})",
            cache_key,
            self.loading_tasks.len()
        );
        cache_key
    }

    /// Remove a loading task
    pub fn finish_loading_task(&mut self, cache_key: &str) {
        if self.loading_tasks.remove(cache_key) {
            tracing::info!(
                "✅ Finished loading task: {} (remaining: {})",
                cache_key,
                self.loading_tasks.len()
            );
        }
    }

    pub fn clear_resources(&mut self) {
        self.resources.clear();
        // Also clear property catalog
        self.property_catalog = crate::app::resource_explorer::PropertyCatalog::new();
    }

    /// Set resources and rebuild property catalog
    ///
    /// This is a convenience method that sets resources and rebuilds the property catalog
    /// in one operation. Use this instead of directly setting `state.resources = ...`
    pub fn set_resources(&mut self, resources: Vec<ResourceEntry>) {
        self.resources = resources;
        self.rebuild_property_catalog();
    }

    /// Rebuild the property catalog from current visible resources
    ///
    /// This should be called after resources are filtered or updated.
    /// The catalog will discover properties from all visible resources.
    pub fn rebuild_property_catalog(&mut self) {
        tracing::debug!(
            "Rebuilding property catalog from {} resources",
            self.resources.len()
        );

        // Rebuild catalog from current resources
        self.property_catalog.rebuild(&self.resources);

        tracing::debug!(
            "Property catalog rebuilt: {} properties discovered",
            self.property_catalog.keys().count()
        );
    }

    /// Rebuild the property catalog from resources in the active selection only
    ///
    /// This filters resources by the current query scope (accounts, regions, resource types)
    /// and rebuilds the property catalog from only those resources.
    /// This should be called when the active selection changes.
    pub fn rebuild_property_catalog_from_active_selection(&mut self) {
        // Filter resources to match current query scope
        let scoped_resources: Vec<ResourceEntry> = self.resources.iter()
            .filter(|resource| {
                // Check if resource matches any selected account
                let account_match = self.query_scope.accounts.is_empty() ||
                    self.query_scope.accounts.iter().any(|a| a.account_id == resource.account_id);

                // Check if resource matches any selected region
                let region_match = self.query_scope.regions.is_empty() ||
                    self.query_scope.regions.iter().any(|r| r.region_code == resource.region);

                // Check if resource matches any selected resource type
                let type_match = self.query_scope.resource_types.is_empty() ||
                    self.query_scope.resource_types.iter().any(|rt| rt.resource_type == resource.resource_type);

                account_match && region_match && type_match
            })
            .cloned()
            .collect();

        tracing::debug!(
            "Rebuilding property catalog from {} scoped resources (out of {} total)",
            scoped_resources.len(),
            self.resources.len()
        );

        // Rebuild catalog from scoped resources
        self.property_catalog.rebuild(&scoped_resources);

        tracing::debug!(
            "Property catalog rebuilt: {} properties discovered from active selection",
            self.property_catalog.keys().count()
        );
    }

    /// Calculate the total memory used by cached queries in bytes
    pub fn calculate_cache_memory_bytes(&self) -> usize {
        self.cached_queries
            .values()
            .flat_map(|resources| resources.iter())
            .map(|resource| resource.estimate_memory_bytes())
            .sum()
    }

    /// Calculate the total memory used by all resources (cache + active) in bytes
    pub fn calculate_total_memory_bytes(&self) -> usize {
        let cache_memory = self.calculate_cache_memory_bytes();
        let active_memory: usize = self.resources
            .iter()
            .map(|resource| resource.estimate_memory_bytes())
            .sum();
        cache_memory + active_memory
    }

    /// Get the number of resources in cache
    pub fn get_cache_resource_count(&self) -> usize {
        self.cached_queries
            .values()
            .map(|resources| resources.len())
            .sum()
    }

    /// Format bytes into human-readable size (KB, MB, GB)
    pub fn format_memory_size(bytes: usize) -> String {
        const KB: usize = 1024;
        const MB: usize = KB * 1024;
        const GB: usize = MB * 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} bytes", bytes)
        }
    }

    /// Get resources filtered by both tag and property filters
    pub fn get_filtered_resources(&self) -> Vec<&ResourceEntry> {
        let mut filtered: Vec<&ResourceEntry> = self.resources.iter().collect();

        // Apply tag filters first
        if !self.tag_filter_group.is_empty() {
            filtered.retain(|resource| self.tag_filter_group.matches(resource));
        }

        // Apply property filters
        if !self.property_filter_group.is_empty() {
            filtered.retain(|resource| {
                self.property_filter_group
                    .matches(&resource.resource_id, &self.property_catalog)
            });
        }

        filtered
    }

    /// Helper method to safely update loading state even if lock contention occurs
    pub fn loading_task_count(&self) -> usize {
        self.loading_tasks.len()
    }

    /// Update tag popularity and badge selector after resources are loaded
    /// This should be called after resources are updated from a query
    pub fn update_tag_popularity(&mut self) {
        // Discover tags from resources (populates tag discovery for autocomplete)
        tracing::info!("🏷️  Starting tag discovery for {} resources", self.resources.len());
        self.tag_discovery.discover_tags(&self.resources);
        tracing::info!("🏷️  Tag discovery complete - {} unique tag keys found", self.tag_discovery.tag_key_count());

        // Analyze resources for tag popularity
        self.tag_popularity.analyze_resources(&self.resources);

        // Extract tag keys from active filters to use as priority keys
        let mut priority_keys = Vec::new();
        self.tag_filter_group
            .collect_filter_tag_keys(&mut priority_keys);

        // Also add tag keys from grouping mode if tag-based
        for key in self.primary_grouping.tag_keys() {
            if !priority_keys.contains(&key) {
                priority_keys.push(key);
            }
        }

        // Update badge selector with priority keys
        self.badge_selector = crate::app::resource_explorer::tag_badges::BadgeSelector::new()
            .with_max_badges(5)
            .with_priority_keys(priority_keys);

        tracing::debug!(
            "Updated tag popularity: {} unique combinations across {} resources",
            self.tag_popularity.unique_combination_count(),
            self.resources.len()
        );

        // Rebuild property catalog
        self.rebuild_property_catalog();
    }

    /// Get count of active presence/absence tag filters
    pub fn tag_presence_filter_count(&self) -> usize {
        let mut count = 0;
        if self.show_only_tagged {
            count += 1;
        }
        if self.show_only_untagged {
            count += 1;
        }
        count
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GroupingMode {
    /// Group resources by AWS account
    ByAccount,
    /// Group resources by AWS region
    ByRegion,
    /// Group resources by AWS resource type (e.g., AWS::EC2::Instance)
    ByResourceType,
    /// Group resources by a single tag key (e.g., "Environment")
    /// Resources without this tag are grouped under "Untagged"
    ByTag(String),
    /// Group resources by multiple tag keys in hierarchical order
    /// Example: ["Environment", "Team", "Project"] creates:
    ///   Production > Backend > API
    ///   Production > Frontend > WebUI
    ///   Staging > Backend > API
    /// Resources missing tags are grouped under "Untagged" at the appropriate level
    ByTagHierarchy(Vec<String>),
    /// Group resources by a single property path (e.g., "instance.state.name")
    /// Resources without this property are grouped under "(not set)"
    ByProperty(String),
    /// Group resources by multiple property paths in hierarchical order
    /// Example: ["instance.state.name", "placement.availability_zone"] creates:
    ///   running > us-east-1a
    ///   running > us-east-1b
    ///   stopped > us-east-1a
    /// Resources missing properties are grouped under "(not set)" at the appropriate level
    ByPropertyHierarchy(Vec<String>),
}

impl GroupingMode {
    pub fn display_name(&self) -> String {
        match self {
            GroupingMode::ByAccount => "Account".to_string(),
            GroupingMode::ByRegion => "Region".to_string(),
            GroupingMode::ByResourceType => "Resource Type".to_string(),
            GroupingMode::ByTag(key) => format!("Tag: {}", key),
            GroupingMode::ByTagHierarchy(keys) => {
                if keys.is_empty() {
                    "Tag Hierarchy (empty)".to_string()
                } else if keys.len() == 1 {
                    format!("Tag: {}", keys[0])
                } else {
                    format!("Tag Hierarchy: {} > ...", keys[0])
                }
            }
            GroupingMode::ByProperty(path) => {
                let display_path = path.replace('.', " > ");
                format!("Property: {}", display_path)
            }
            GroupingMode::ByPropertyHierarchy(paths) => {
                if paths.is_empty() {
                    "Property Hierarchy (empty)".to_string()
                } else if paths.len() == 1 {
                    let display_path = paths[0].replace('.', " > ");
                    format!("Property: {}", display_path)
                } else {
                    let display_path = paths[0].replace('.', " > ");
                    format!("Property Hierarchy: {} > ...", display_path)
                }
            }
        }
    }

    /// Get the default grouping modes (non-tag modes)
    pub fn default_modes() -> Vec<GroupingMode> {
        vec![
            GroupingMode::ByAccount,
            GroupingMode::ByRegion,
            GroupingMode::ByResourceType,
        ]
    }

    /// Get all standard grouping modes (backward compatibility)
    /// Note: Tag-based modes are created dynamically based on discovered tags
    pub fn all_modes() -> Vec<GroupingMode> {
        Self::default_modes()
    }

    /// Check if this is a tag-based grouping mode
    pub fn is_tag_based(&self) -> bool {
        matches!(self, GroupingMode::ByTag(_) | GroupingMode::ByTagHierarchy(_))
    }

    /// Get the tag keys used for grouping (if tag-based)
    pub fn tag_keys(&self) -> Vec<String> {
        match self {
            GroupingMode::ByTag(key) => vec![key.clone()],
            GroupingMode::ByTagHierarchy(keys) => keys.clone(),
            _ => Vec::new(),
        }
    }

    /// Check if this grouping mode is valid
    pub fn is_valid(&self) -> bool {
        match self {
            GroupingMode::ByAccount | GroupingMode::ByRegion | GroupingMode::ByResourceType => {
                true
            }
            GroupingMode::ByTag(key) => !key.is_empty(),
            GroupingMode::ByTagHierarchy(keys) => {
                !keys.is_empty() && keys.iter().all(|k| !k.is_empty())
            }
            GroupingMode::ByProperty(path) => !path.is_empty(),
            GroupingMode::ByPropertyHierarchy(paths) => {
                !paths.is_empty() && paths.iter().all(|p| !p.is_empty())
            }
        }
    }

    /// Get a short label for UI display (max 20 chars)
    pub fn short_label(&self) -> String {
        match self {
            GroupingMode::ByAccount => "Account".to_string(),
            GroupingMode::ByRegion => "Region".to_string(),
            GroupingMode::ByResourceType => "Type".to_string(),
            GroupingMode::ByTag(key) => {
                if key.len() > 15 {
                    format!("{}...", &key[..12])
                } else {
                    key.clone()
                }
            }
            GroupingMode::ByTagHierarchy(keys) => {
                if keys.is_empty() {
                    "Hierarchy".to_string()
                } else {
                    let first = &keys[0];
                    if first.len() > 12 {
                        format!("{}...+{}", &first[..9], keys.len() - 1)
                    } else if keys.len() > 1 {
                        format!("{}+{}", first, keys.len() - 1)
                    } else {
                        first.clone()
                    }
                }
            }
            GroupingMode::ByProperty(path) => {
                // Extract last segment of path for brevity
                let last_segment = path.split('.').next_back().unwrap_or(path);
                if last_segment.len() > 15 {
                    format!("{}...", &last_segment[..12])
                } else {
                    last_segment.to_string()
                }
            }
            GroupingMode::ByPropertyHierarchy(paths) => {
                if paths.is_empty() {
                    "PropHier".to_string()
                } else {
                    let first = paths[0].split('.').next_back().unwrap_or(&paths[0]);
                    if first.len() > 12 {
                        format!("{}...+{}", &first[..9], paths.len() - 1)
                    } else if paths.len() > 1 {
                        format!("{}+{}", first, paths.len() - 1)
                    } else {
                        first.to_string()
                    }
                }
            }
        }
    }
}

// Color assignment utilities
impl AccountSelection {
    pub fn new(account_id: String, display_name: String) -> Self {
        let color = assign_account_color(&account_id);
        Self {
            account_id,
            display_name,
            color,
        }
    }
}

impl RegionSelection {
    pub fn new(region_code: String, display_name: String) -> Self {
        let color = assign_region_color(&region_code);
        Self {
            region_code,
            display_name,
            color,
        }
    }
}

impl ResourceTypeSelection {
    pub fn new(resource_type: String, display_name: String, service_name: String) -> Self {
        Self {
            resource_type,
            display_name,
            service_name,
        }
    }
}

/// Assign consistent colors to accounts based on account ID
pub fn assign_account_color(account_id: &str) -> egui::Color32 {
    // Use the same seeded random approach as the full color generator
    use crate::app::resource_explorer::colors::AwsColorGenerator;

    // Create a temporary color generator to get consistent results
    let generator = AwsColorGenerator::new();
    generator.get_account_color(account_id)
}

/// Assign consistent colors to regions based on region code
pub fn assign_region_color(region_code: &str) -> egui::Color32 {
    // Use the same seeded random approach as the full color generator
    use crate::app::resource_explorer::colors::AwsColorGenerator;

    // Create a temporary color generator to get consistent results
    let generator = AwsColorGenerator::new();
    generator.get_region_color(region_code)
}
