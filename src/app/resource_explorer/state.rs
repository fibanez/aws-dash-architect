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
    pub account_color: egui::Color32,             // Consistent color for account
    pub region_color: egui::Color32,              // Consistent color for region
    pub query_timestamp: DateTime<Utc>,           // When this resource data was fetched
}

impl ResourceEntry {
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
    AttachedTo,      // EBS attached to EC2
    MemberOf,        // User member of Group
    DeployedIn,      // MQ Broker deployed in Subnet
    ProtectedBy,     // MQ Broker protected by Security Group
    DeadLetterQueue, // SQS Queue uses another queue as DLQ
    ServesAsDlq,     // SQS Queue serves as DLQ for another queue
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
    pub loading_tasks: HashSet<String>, // Track active queries
    pub cached_queries: HashMap<String, Vec<ResourceEntry>>, // Session cache
    pub show_refresh_dialog: bool,
    pub show_account_dialog: bool,
    pub show_region_dialog: bool,
    pub show_resource_type_dialog: bool,
    pub stale_data_threshold_minutes: i64, // Data older than this is considered stale
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
            loading_tasks: HashSet::new(),
            cached_queries: HashMap::new(),
            show_refresh_dialog: false,
            show_account_dialog: false,
            show_region_dialog: false,
            show_resource_type_dialog: false,
            stale_data_threshold_minutes: 15, // Consider data stale after 15 minutes
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
    }

    /// Helper method to safely update loading state even if lock contention occurs
    pub fn loading_task_count(&self) -> usize {
        self.loading_tasks.len()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GroupingMode {
    ByAccount,
    ByRegion,
    ByResourceType,
}

impl GroupingMode {
    pub fn display_name(&self) -> &'static str {
        match self {
            GroupingMode::ByAccount => "Account",
            GroupingMode::ByRegion => "Region",
            GroupingMode::ByResourceType => "Resource Type",
        }
    }

    pub fn all_modes() -> Vec<GroupingMode> {
        vec![
            GroupingMode::ByAccount,
            GroupingMode::ByRegion,
            GroupingMode::ByResourceType,
        ]
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
