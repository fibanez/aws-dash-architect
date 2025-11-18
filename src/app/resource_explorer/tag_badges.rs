use crate::app::resource_explorer::state::ResourceEntry;
use std::collections::HashMap;

/// A tag key-value pair
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TagCombination {
    pub key: String,
    pub value: String,
}

impl TagCombination {
    pub fn new(key: String, value: String) -> Self {
        Self { key, value }
    }

    /// Format for display: "Key: Value"
    pub fn display_name(&self) -> String {
        format!("{}: {}", self.key, self.value)
    }

    /// Short format for compact display
    pub fn short_display(&self, max_len: usize) -> String {
        let display = self.display_name();
        if display.len() > max_len {
            format!("{}...", &display[..max_len.saturating_sub(3)])
        } else {
            display
        }
    }
}

/// Tracks popularity of tag combinations across resources
#[derive(Debug, Clone)]
pub struct TagPopularityTracker {
    /// Map of tag combination to occurrence count
    tag_counts: HashMap<TagCombination, usize>,
    /// Total number of resources analyzed
    total_resources: usize,
}

impl Default for TagPopularityTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl TagPopularityTracker {
    pub fn new() -> Self {
        Self {
            tag_counts: HashMap::new(),
            total_resources: 0,
        }
    }

    /// Analyze resources and track tag popularity
    pub fn analyze_resources(&mut self, resources: &[ResourceEntry]) {
        self.tag_counts.clear();
        self.total_resources = resources.len();

        for resource in resources {
            for tag in &resource.tags {
                let combination = TagCombination::new(tag.key.clone(), tag.value.clone());
                *self.tag_counts.entry(combination).or_insert(0) += 1;
            }
        }

        tracing::debug!(
            "Tag popularity analysis complete: {} unique combinations across {} resources",
            self.tag_counts.len(),
            self.total_resources
        );
    }

    /// Get the top N most popular tag combinations
    pub fn get_top_tags(&self, n: usize) -> Vec<(TagCombination, usize)> {
        let mut sorted: Vec<(TagCombination, usize)> = self
            .tag_counts
            .iter()
            .map(|(combo, count)| (combo.clone(), *count))
            .collect();

        // Sort by count (descending), then by key and value (ascending)
        sorted.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then(a.0.key.cmp(&b.0.key))
                .then(a.0.value.cmp(&b.0.value))
        });

        sorted.into_iter().take(n).collect()
    }

    /// Get count for a specific tag combination
    pub fn get_count(&self, combination: &TagCombination) -> usize {
        self.tag_counts.get(combination).copied().unwrap_or(0)
    }

    /// Get popularity percentage for a tag combination
    pub fn get_percentage(&self, combination: &TagCombination) -> f64 {
        if self.total_resources == 0 {
            0.0
        } else {
            let count = self.get_count(combination);
            (count as f64 / self.total_resources as f64) * 100.0
        }
    }

    /// Check if a tag combination is in the top N
    pub fn is_top_tag(&self, combination: &TagCombination, n: usize) -> bool {
        let top_tags = self.get_top_tags(n);
        top_tags.iter().any(|(combo, _)| combo == combination)
    }

    /// Get total number of unique tag combinations
    pub fn unique_combination_count(&self) -> usize {
        self.tag_counts.len()
    }
}

/// Badge selection strategy for displaying on resources
#[derive(Debug, Clone)]
pub struct BadgeSelector {
    /// Maximum number of badges to display per resource
    max_badges: usize,
    /// Always show these tag keys (e.g., active filters)
    priority_keys: Vec<String>,
}

impl Default for BadgeSelector {
    fn default() -> Self {
        Self::new()
    }
}

impl BadgeSelector {
    pub fn new() -> Self {
        Self {
            max_badges: 5, // Default: show max 5 badges per resource
            priority_keys: Vec::new(),
        }
    }

    /// Set maximum number of badges per resource
    pub fn with_max_badges(mut self, max: usize) -> Self {
        self.max_badges = max;
        self
    }

    /// Set priority tag keys (always shown if present)
    pub fn with_priority_keys(mut self, keys: Vec<String>) -> Self {
        self.priority_keys = keys;
        self
    }

    /// Select which tags to display as badges for a resource
    ///
    /// Strategy:
    /// 1. Always include tags with priority keys (e.g., active filters)
    /// 2. Fill remaining slots with top N popular tags
    /// 3. Limit total to max_badges
    pub fn select_badges(
        &self,
        resource: &ResourceEntry,
        popularity: &TagPopularityTracker,
        top_n: usize,
    ) -> Vec<TagCombination> {
        let mut selected = Vec::new();

        // Get top popular tag combinations
        let top_tags = popularity.get_top_tags(top_n);
        let top_combinations: Vec<TagCombination> = top_tags.into_iter().map(|(c, _)| c).collect();

        // First, add priority tags (from filters/grouping)
        for tag in &resource.tags {
            if self.priority_keys.contains(&tag.key) {
                let combination = TagCombination::new(tag.key.clone(), tag.value.clone());
                if !selected.contains(&combination) {
                    selected.push(combination);
                    if selected.len() >= self.max_badges {
                        return selected;
                    }
                }
            }
        }

        // Then, add top popular tags that aren't already selected
        for tag in &resource.tags {
            let combination = TagCombination::new(tag.key.clone(), tag.value.clone());

            // Skip if already selected
            if selected.contains(&combination) {
                continue;
            }

            // Add if it's in the top N popular tags
            if top_combinations.contains(&combination) {
                selected.push(combination);
                if selected.len() >= self.max_badges {
                    return selected;
                }
            }
        }

        selected
    }

    /// Get the maximum number of badges
    pub fn max_badges(&self) -> usize {
        self.max_badges
    }

    /// Get priority tag keys
    pub fn priority_keys(&self) -> &[String] {
        &self.priority_keys
    }
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
            raw_properties: serde_json::json!({}),
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
    fn test_tag_popularity_tracking() {
        let mut tracker = TagPopularityTracker::new();

        let resources = vec![
            create_test_resource(vec![("Environment", "Production"), ("Team", "Backend")]),
            create_test_resource(vec![("Environment", "Production"), ("Team", "Frontend")]),
            create_test_resource(vec![("Environment", "Staging"), ("Team", "Backend")]),
        ];

        tracker.analyze_resources(&resources);

        // Environment:Production should be most popular (appears twice)
        let top_tags = tracker.get_top_tags(1);
        assert_eq!(top_tags.len(), 1);
        assert_eq!(top_tags[0].0.key, "Environment");
        assert_eq!(top_tags[0].0.value, "Production");
        assert_eq!(top_tags[0].1, 2);
    }

    #[test]
    fn test_badge_selector_priority() {
        let mut tracker = TagPopularityTracker::new();

        let resources = vec![
            create_test_resource(vec![("Environment", "Production"), ("Owner", "Alice")]),
            create_test_resource(vec![("Environment", "Production"), ("Owner", "Bob")]),
            create_test_resource(vec![("Environment", "Staging"), ("Owner", "Alice")]),
        ];

        tracker.analyze_resources(&resources);

        let selector = BadgeSelector::new()
            .with_max_badges(2)
            .with_priority_keys(vec!["Owner".to_string()]);

        // Owner should be selected first (priority), then Environment:Production (popular)
        let resource = &resources[0]; // Has Owner:Alice and Environment:Production
        let badges = selector.select_badges(resource, &tracker, 10);

        assert_eq!(badges.len(), 2);
        assert_eq!(badges[0].key, "Owner"); // Priority tag first
        assert_eq!(badges[1].key, "Environment"); // Popular tag second
    }

    #[test]
    fn test_badge_selector_limit() {
        let mut tracker = TagPopularityTracker::new();

        let resource = create_test_resource(vec![
            ("Tag1", "Value1"),
            ("Tag2", "Value2"),
            ("Tag3", "Value3"),
            ("Tag4", "Value4"),
            ("Tag5", "Value5"),
            ("Tag6", "Value6"),
        ]);

        tracker.analyze_resources(&[resource.clone()]);

        let selector = BadgeSelector::new().with_max_badges(3);

        let badges = selector.select_badges(&resource, &tracker, 10);

        // Should be limited to max 3 badges
        assert!(badges.len() <= 3);
    }
}
