use crate::app::resource_explorer::state::ResourceEntry;
use std::collections::{HashMap, HashSet};

/// Metadata about a discovered tag key
#[derive(Debug, Clone)]
pub struct TagMetadata {
    /// The tag key
    pub key: String,
    /// All unique values seen for this key
    pub values: HashSet<String>,
    /// Count of resources with this tag key
    pub resource_count: usize,
    /// Total number of times this tag appears across all resources
    pub occurrence_count: usize,
}

impl TagMetadata {
    pub fn new(key: String) -> Self {
        Self {
            key,
            values: HashSet::new(),
            resource_count: 0,
            occurrence_count: 0,
        }
    }

    /// Add a value occurrence for this tag
    pub fn add_value(&mut self, value: String) {
        self.values.insert(value);
        self.occurrence_count += 1;
    }

    /// Increment the resource count
    pub fn increment_resource_count(&mut self) {
        self.resource_count += 1;
    }

    /// Get the number of unique values for this tag
    pub fn value_count(&self) -> usize {
        self.values.len()
    }

    /// Check if this tag has multiple values
    pub fn has_multiple_values(&self) -> bool {
        self.values.len() > 1
    }

    /// Get sorted list of values (most common approach: alphabetical)
    pub fn get_sorted_values(&self) -> Vec<String> {
        let mut values: Vec<String> = self.values.iter().cloned().collect();
        values.sort();
        values
    }
}

/// Tag discovery service - analyzes resources and extracts tag metadata
#[derive(Debug, Default, Clone)]
pub struct TagDiscovery {
    /// Map of tag key to its metadata
    tag_metadata: HashMap<String, TagMetadata>,
    /// Total number of resources analyzed
    total_resources: usize,
    /// Number of resources with at least one tag
    tagged_resource_count: usize,
}

impl TagDiscovery {
    pub fn new() -> Self {
        Self {
            tag_metadata: HashMap::new(),
            total_resources: 0,
            tagged_resource_count: 0,
        }
    }

    /// Discover tags from a set of resources
    pub fn discover_tags(&mut self, resources: &[ResourceEntry]) {
        // Reset counters
        self.tag_metadata.clear();
        self.total_resources = resources.len();
        self.tagged_resource_count = 0;

        // Analyze each resource
        for resource in resources {
            if !resource.tags.is_empty() {
                self.tagged_resource_count += 1;

                // Track each tag on this resource
                let mut resource_tags = HashSet::new();

                for tag in &resource.tags {
                    // Get or create metadata for this tag key
                    let metadata = self
                        .tag_metadata
                        .entry(tag.key.clone())
                        .or_insert_with(|| TagMetadata::new(tag.key.clone()));

                    // Add this value
                    metadata.add_value(tag.value.clone());

                    // Track unique tags per resource
                    resource_tags.insert(tag.key.clone());
                }

                // Increment resource count for each unique tag on this resource
                for tag_key in resource_tags {
                    if let Some(metadata) = self.tag_metadata.get_mut(&tag_key) {
                        metadata.increment_resource_count();
                    }
                }
            }
        }

        tracing::info!(
            "🏷️  Tag discovery complete: {} unique tag keys, {} tagged resources of {} total",
            self.tag_metadata.len(),
            self.tagged_resource_count,
            self.total_resources
        );
    }

    /// Get all discovered tag keys (sorted alphabetically)
    pub fn get_tag_keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.tag_metadata.keys().cloned().collect();
        keys.sort();
        keys
    }

    /// Get metadata for a specific tag key
    pub fn get_tag_metadata(&self, key: &str) -> Option<&TagMetadata> {
        self.tag_metadata.get(key)
    }

    /// Get all tag metadata (sorted by key)
    pub fn get_all_metadata(&self) -> Vec<&TagMetadata> {
        let mut metadata: Vec<&TagMetadata> = self.tag_metadata.values().collect();
        metadata.sort_by(|a, b| a.key.cmp(&b.key));
        metadata
    }

    /// Get the number of unique tag keys discovered
    pub fn tag_key_count(&self) -> usize {
        self.tag_metadata.len()
    }

    /// Get the total number of resources analyzed
    pub fn total_resource_count(&self) -> usize {
        self.total_resources
    }

    /// Get the number of resources with at least one tag
    pub fn tagged_resource_count(&self) -> usize {
        self.tagged_resource_count
    }

    /// Get the number of resources without any tags
    pub fn untagged_resource_count(&self) -> usize {
        self.total_resources
            .saturating_sub(self.tagged_resource_count)
    }

    /// Get percentage of resources that have tags
    pub fn tag_coverage_percentage(&self) -> f64 {
        if self.total_resources == 0 {
            0.0
        } else {
            (self.tagged_resource_count as f64 / self.total_resources as f64) * 100.0
        }
    }

    /// Get all unique values for a specific tag key (sorted)
    pub fn get_tag_values(&self, key: &str) -> Vec<String> {
        self.tag_metadata
            .get(key)
            .map(|m| m.get_sorted_values())
            .unwrap_or_default()
    }

    /// Get tag keys sorted by resource count (most common first)
    pub fn get_tag_keys_by_popularity(&self) -> Vec<(String, usize)> {
        let mut keys: Vec<(String, usize)> = self
            .tag_metadata
            .iter()
            .map(|(key, metadata)| (key.clone(), metadata.resource_count))
            .collect();

        keys.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        keys
    }

    /// Get tag keys that have multiple values (useful for grouping)
    pub fn get_multi_value_tag_keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self
            .tag_metadata
            .iter()
            .filter(|(_, metadata)| metadata.has_multiple_values())
            .map(|(key, _)| key.clone())
            .collect();

        keys.sort();
        keys
    }

    /// Check if a tag key exists in the discovered tags
    pub fn has_tag_key(&self, key: &str) -> bool {
        self.tag_metadata.contains_key(key)
    }

    /// Get statistics for a tag key
    pub fn get_tag_stats(&self, key: &str) -> Option<TagStats> {
        self.tag_metadata.get(key).map(|metadata| TagStats {
            tag_key: key.to_string(),
            unique_values: metadata.value_count(),
            resource_count: metadata.resource_count,
            occurrence_count: metadata.occurrence_count,
            coverage_percentage: if self.total_resources > 0 {
                (metadata.resource_count as f64 / self.total_resources as f64) * 100.0
            } else {
                0.0
            },
        })
    }

    /// Get overall tag statistics
    pub fn get_overall_stats(&self) -> OverallTagStats {
        OverallTagStats {
            total_resources: self.total_resources,
            tagged_resources: self.tagged_resource_count,
            untagged_resources: self.untagged_resource_count(),
            unique_tag_keys: self.tag_key_count(),
            tag_coverage_percentage: self.tag_coverage_percentage(),
        }
    }
}

/// Statistics for a specific tag key
#[derive(Debug, Clone)]
pub struct TagStats {
    pub tag_key: String,
    pub unique_values: usize,
    pub resource_count: usize,
    pub occurrence_count: usize,
    pub coverage_percentage: f64,
}

/// Overall tag statistics across all resources
#[derive(Debug, Clone)]
pub struct OverallTagStats {
    pub total_resources: usize,
    pub tagged_resources: usize,
    pub untagged_resources: usize,
    pub unique_tag_keys: usize,
    pub tag_coverage_percentage: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::resource_explorer::state::ResourceTag;

    fn create_test_resource(tags: Vec<(&str, &str)>) -> ResourceEntry {
        ResourceEntry {
            resource_type: "AWS::EC2::Instance".to_string(),
            account_id: "123456789012".to_string(),
            region: "us-east-1".to_string(),
            resource_id: "i-1234567890abcdef0".to_string(),
            display_name: "test-instance".to_string(),
            status: Some("running".to_string()),
            properties: serde_json::json!({}),
            properties: serde_json::json!({}),
            detailed_properties: None,
            detailed_timestamp: None,
            tags: tags
                .into_iter()
                .map(|(k, v)| ResourceTag {
                    key: k.to_string(),
                    value: v.to_string(),
                })
                .collect(),
            relationships: Vec::new(),
            parent_resource_id: None,
            parent_resource_type: None,
            is_child_resource: false,
            account_color: egui::Color32::WHITE,
            region_color: egui::Color32::WHITE,
            query_timestamp: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_tag_discovery_empty() {
        let mut discovery = TagDiscovery::new();
        discovery.discover_tags(&[]);

        assert_eq!(discovery.tag_key_count(), 0);
        assert_eq!(discovery.total_resource_count(), 0);
        assert_eq!(discovery.tagged_resource_count(), 0);
    }

    #[test]
    fn test_tag_discovery_single_resource() {
        let mut discovery = TagDiscovery::new();
        let resources = vec![create_test_resource(vec![
            ("Environment", "Production"),
            ("Team", "Backend"),
        ])];

        discovery.discover_tags(&resources);

        assert_eq!(discovery.tag_key_count(), 2);
        assert_eq!(discovery.total_resource_count(), 1);
        assert_eq!(discovery.tagged_resource_count(), 1);
        assert!(discovery.has_tag_key("Environment"));
        assert!(discovery.has_tag_key("Team"));
    }

    #[test]
    fn test_tag_discovery_multiple_values() {
        let mut discovery = TagDiscovery::new();
        let resources = vec![
            create_test_resource(vec![("Environment", "Production")]),
            create_test_resource(vec![("Environment", "Staging")]),
            create_test_resource(vec![("Environment", "Development")]),
        ];

        discovery.discover_tags(&resources);

        let env_metadata = discovery.get_tag_metadata("Environment").unwrap();
        assert_eq!(env_metadata.value_count(), 3);
        assert_eq!(env_metadata.resource_count, 3);
        assert!(env_metadata.has_multiple_values());
    }

    #[test]
    fn test_tag_coverage_percentage() {
        let mut discovery = TagDiscovery::new();
        let resources = vec![
            create_test_resource(vec![("Environment", "Production")]),
            create_test_resource(vec![("Environment", "Staging")]),
            create_test_resource(vec![]), // Untagged resource
        ];

        discovery.discover_tags(&resources);

        assert_eq!(discovery.total_resource_count(), 3);
        assert_eq!(discovery.tagged_resource_count(), 2);
        assert_eq!(discovery.untagged_resource_count(), 1);
        assert!((discovery.tag_coverage_percentage() - 66.666).abs() < 0.01);
    }
}
