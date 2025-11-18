//! Tag Filter System Unit Tests
//!
//! Comprehensive unit tests for the tag filtering system, covering all 11 filter types,
//! boolean logic, and edge cases. These tests ensure the tag filtering logic works correctly
//! across various scenarios including missing tags, empty values, and complex nested groups.
//!
//! # Test Coverage
//!
//! - **Individual Filter Types**: Tests for all 11 filter operations
//! - **Filter Validation**: Ensures filters are properly configured before use
//! - **Resource Matching**: Verifies filters correctly match/reject resources
//! - **Boolean Logic**: Tests AND/OR operators in filter groups
//! - **Nested Groups**: Complex multi-level filter combinations
//! - **Edge Cases**: Missing tags, empty values, invalid regex patterns
//!
//! # Filter Types Tested
//!
//! 1. Equals - Tag value equals specific value(s)
//! 2. NotEquals - Tag value does not equal specific value(s)
//! 3. Contains - Tag value contains substring
//! 4. NotContains - Tag value does not contain substring
//! 5. StartsWith - Tag value starts with prefix
//! 6. EndsWith - Tag value ends with suffix
//! 7. In - Tag value is in list of values
//! 8. NotIn - Tag value is not in list of values
//! 9. Exists - Tag key exists (any value)
//! 10. NotExists - Tag key does not exist
//! 11. Regex - Tag value matches regex pattern

use awsdash::app::resource_explorer::state::{
    BooleanOperator, ResourceEntry, ResourceTag, TagFilter, TagFilterGroup, TagFilterType,
};
use chrono::Utc;

/// Helper function to create a test resource with specific tags
fn create_test_resource(tags: Vec<(&str, &str)>) -> ResourceEntry {
    ResourceEntry {
        resource_type: "AWS::EC2::Instance".to_string(),
        account_id: "123456789012".to_string(),
        region: "us-east-1".to_string(),
        resource_id: "i-1234567890abcdef0".to_string(),
        display_name: "Test Instance".to_string(),
        status: Some("running".to_string()),
        properties: serde_json::json!({}),
        raw_properties: serde_json::json!({}),
        detailed_properties: None,
        detailed_timestamp: None,
        tags: tags
            .into_iter()
            .map(|(key, value)| ResourceTag {
                key: key.to_string(),
                value: value.to_string(),
            })
            .collect(),
        relationships: Vec::new(),
        parent_resource_id: None,
        parent_resource_type: None,
        is_child_resource: false,
        account_color: egui::Color32::from_rgb(100, 100, 100),
        region_color: egui::Color32::from_rgb(150, 150, 150),
        query_timestamp: Utc::now(),
    }
}

// ============================================================================
// TagFilterType Tests - Equals
// ============================================================================

#[test]
fn test_equals_filter_matches_exact_value() {
    let resource = create_test_resource(vec![("Environment", "Production")]);
    let filter = TagFilter::new("Environment".to_string(), TagFilterType::Equals)
        .with_values(vec!["Production".to_string()]);

    assert!(filter.matches(&resource));
}

#[test]
fn test_equals_filter_rejects_different_value() {
    let resource = create_test_resource(vec![("Environment", "Staging")]);
    let filter = TagFilter::new("Environment".to_string(), TagFilterType::Equals)
        .with_values(vec!["Production".to_string()]);

    assert!(!filter.matches(&resource));
}

#[test]
fn test_equals_filter_matches_one_of_multiple_values() {
    let resource = create_test_resource(vec![("Environment", "Staging")]);
    let filter = TagFilter::new("Environment".to_string(), TagFilterType::Equals)
        .with_values(vec!["Production".to_string(), "Staging".to_string()]);

    assert!(filter.matches(&resource));
}

#[test]
fn test_equals_filter_rejects_missing_tag() {
    let resource = create_test_resource(vec![("Team", "Backend")]);
    let filter = TagFilter::new("Environment".to_string(), TagFilterType::Equals)
        .with_values(vec!["Production".to_string()]);

    assert!(!filter.matches(&resource));
}

// ============================================================================
// TagFilterType Tests - NotEquals
// ============================================================================

#[test]
fn test_not_equals_filter_rejects_exact_value() {
    let resource = create_test_resource(vec![("Environment", "Production")]);
    let filter = TagFilter::new("Environment".to_string(), TagFilterType::NotEquals)
        .with_values(vec!["Production".to_string()]);

    assert!(!filter.matches(&resource));
}

#[test]
fn test_not_equals_filter_matches_different_value() {
    let resource = create_test_resource(vec![("Environment", "Staging")]);
    let filter = TagFilter::new("Environment".to_string(), TagFilterType::NotEquals)
        .with_values(vec!["Production".to_string()]);

    assert!(filter.matches(&resource));
}

#[test]
fn test_not_equals_filter_matches_missing_tag() {
    let resource = create_test_resource(vec![("Team", "Backend")]);
    let filter = TagFilter::new("Environment".to_string(), TagFilterType::NotEquals)
        .with_values(vec!["Production".to_string()]);

    // Tag doesn't exist, so it's not equal to any value
    assert!(filter.matches(&resource));
}

// ============================================================================
// TagFilterType Tests - Contains
// ============================================================================

#[test]
fn test_contains_filter_matches_substring() {
    let resource = create_test_resource(vec![("Application", "api-gateway-prod")]);
    let filter = TagFilter::new("Application".to_string(), TagFilterType::Contains)
        .with_values(vec!["gateway".to_string()]);

    assert!(filter.matches(&resource));
}

#[test]
fn test_contains_filter_rejects_non_substring() {
    let resource = create_test_resource(vec![("Application", "api-server-prod")]);
    let filter = TagFilter::new("Application".to_string(), TagFilterType::Contains)
        .with_values(vec!["gateway".to_string()]);

    assert!(!filter.matches(&resource));
}

#[test]
fn test_contains_filter_is_case_sensitive() {
    let resource = create_test_resource(vec![("Application", "API-GATEWAY")]);
    let filter = TagFilter::new("Application".to_string(), TagFilterType::Contains)
        .with_values(vec!["gateway".to_string()]);

    assert!(!filter.matches(&resource));
}

// ============================================================================
// TagFilterType Tests - NotContains
// ============================================================================

#[test]
fn test_not_contains_filter_rejects_substring() {
    let resource = create_test_resource(vec![("Application", "api-gateway-prod")]);
    let filter = TagFilter::new("Application".to_string(), TagFilterType::NotContains)
        .with_values(vec!["gateway".to_string()]);

    assert!(!filter.matches(&resource));
}

#[test]
fn test_not_contains_filter_matches_non_substring() {
    let resource = create_test_resource(vec![("Application", "api-server-prod")]);
    let filter = TagFilter::new("Application".to_string(), TagFilterType::NotContains)
        .with_values(vec!["gateway".to_string()]);

    assert!(filter.matches(&resource));
}

#[test]
fn test_not_contains_filter_matches_missing_tag() {
    let resource = create_test_resource(vec![("Team", "Backend")]);
    let filter = TagFilter::new("Application".to_string(), TagFilterType::NotContains)
        .with_values(vec!["gateway".to_string()]);

    // Tag doesn't exist, so it doesn't contain anything
    assert!(filter.matches(&resource));
}

// ============================================================================
// TagFilterType Tests - StartsWith
// ============================================================================

#[test]
fn test_starts_with_filter_matches_prefix() {
    let resource = create_test_resource(vec![("Application", "prod-api-gateway")]);
    let filter = TagFilter::new("Application".to_string(), TagFilterType::StartsWith)
        .with_values(vec!["prod-".to_string()]);

    assert!(filter.matches(&resource));
}

#[test]
fn test_starts_with_filter_rejects_non_prefix() {
    let resource = create_test_resource(vec![("Application", "staging-api-gateway")]);
    let filter = TagFilter::new("Application".to_string(), TagFilterType::StartsWith)
        .with_values(vec!["prod-".to_string()]);

    assert!(!filter.matches(&resource));
}

#[test]
fn test_starts_with_filter_rejects_suffix_match() {
    let resource = create_test_resource(vec![("Application", "api-gateway-prod")]);
    let filter = TagFilter::new("Application".to_string(), TagFilterType::StartsWith)
        .with_values(vec!["prod".to_string()]);

    assert!(!filter.matches(&resource));
}

// ============================================================================
// TagFilterType Tests - EndsWith
// ============================================================================

#[test]
fn test_ends_with_filter_matches_suffix() {
    let resource = create_test_resource(vec![("Application", "api-gateway-prod")]);
    let filter = TagFilter::new("Application".to_string(), TagFilterType::EndsWith)
        .with_values(vec!["-prod".to_string()]);

    assert!(filter.matches(&resource));
}

#[test]
fn test_ends_with_filter_rejects_non_suffix() {
    let resource = create_test_resource(vec![("Application", "api-gateway-staging")]);
    let filter = TagFilter::new("Application".to_string(), TagFilterType::EndsWith)
        .with_values(vec!["-prod".to_string()]);

    assert!(!filter.matches(&resource));
}

#[test]
fn test_ends_with_filter_rejects_prefix_match() {
    let resource = create_test_resource(vec![("Application", "prod-api-gateway")]);
    let filter = TagFilter::new("Application".to_string(), TagFilterType::EndsWith)
        .with_values(vec!["prod".to_string()]);

    assert!(!filter.matches(&resource));
}

// ============================================================================
// TagFilterType Tests - In
// ============================================================================

#[test]
fn test_in_filter_matches_value_in_list() {
    let resource = create_test_resource(vec![("Environment", "Staging")]);
    let filter = TagFilter::new("Environment".to_string(), TagFilterType::In).with_values(vec![
        "Production".to_string(),
        "Staging".to_string(),
        "Development".to_string(),
    ]);

    assert!(filter.matches(&resource));
}

#[test]
fn test_in_filter_rejects_value_not_in_list() {
    let resource = create_test_resource(vec![("Environment", "QA")]);
    let filter = TagFilter::new("Environment".to_string(), TagFilterType::In).with_values(vec![
        "Production".to_string(),
        "Staging".to_string(),
        "Development".to_string(),
    ]);

    assert!(!filter.matches(&resource));
}

#[test]
fn test_in_filter_rejects_missing_tag() {
    let resource = create_test_resource(vec![("Team", "Backend")]);
    let filter = TagFilter::new("Environment".to_string(), TagFilterType::In).with_values(vec![
        "Production".to_string(),
        "Staging".to_string(),
    ]);

    assert!(!filter.matches(&resource));
}

// ============================================================================
// TagFilterType Tests - NotIn
// ============================================================================

#[test]
fn test_not_in_filter_rejects_value_in_list() {
    let resource = create_test_resource(vec![("Environment", "Staging")]);
    let filter = TagFilter::new("Environment".to_string(), TagFilterType::NotIn).with_values(vec![
        "Production".to_string(),
        "Staging".to_string(),
        "Development".to_string(),
    ]);

    assert!(!filter.matches(&resource));
}

#[test]
fn test_not_in_filter_matches_value_not_in_list() {
    let resource = create_test_resource(vec![("Environment", "QA")]);
    let filter = TagFilter::new("Environment".to_string(), TagFilterType::NotIn).with_values(vec![
        "Production".to_string(),
        "Staging".to_string(),
        "Development".to_string(),
    ]);

    assert!(filter.matches(&resource));
}

#[test]
fn test_not_in_filter_matches_missing_tag() {
    let resource = create_test_resource(vec![("Team", "Backend")]);
    let filter = TagFilter::new("Environment".to_string(), TagFilterType::NotIn).with_values(vec![
        "Production".to_string(),
        "Staging".to_string(),
    ]);

    // Tag doesn't exist, so it's not in any list
    assert!(filter.matches(&resource));
}

// ============================================================================
// TagFilterType Tests - Exists
// ============================================================================

#[test]
fn test_exists_filter_matches_tag_with_any_value() {
    let resource = create_test_resource(vec![("Environment", "Production")]);
    let filter = TagFilter::new("Environment".to_string(), TagFilterType::Exists);

    assert!(filter.matches(&resource));
}

#[test]
fn test_exists_filter_matches_tag_with_empty_value() {
    let resource = create_test_resource(vec![("Environment", "")]);
    let filter = TagFilter::new("Environment".to_string(), TagFilterType::Exists);

    assert!(filter.matches(&resource));
}

#[test]
fn test_exists_filter_rejects_missing_tag() {
    let resource = create_test_resource(vec![("Team", "Backend")]);
    let filter = TagFilter::new("Environment".to_string(), TagFilterType::Exists);

    assert!(!filter.matches(&resource));
}

// ============================================================================
// TagFilterType Tests - NotExists
// ============================================================================

#[test]
fn test_not_exists_filter_rejects_tag_with_value() {
    let resource = create_test_resource(vec![("Environment", "Production")]);
    let filter = TagFilter::new("Environment".to_string(), TagFilterType::NotExists);

    assert!(!filter.matches(&resource));
}

#[test]
fn test_not_exists_filter_matches_missing_tag() {
    let resource = create_test_resource(vec![("Team", "Backend")]);
    let filter = TagFilter::new("Environment".to_string(), TagFilterType::NotExists);

    assert!(filter.matches(&resource));
}

// ============================================================================
// TagFilterType Tests - Regex
// ============================================================================

#[test]
fn test_regex_filter_matches_pattern() {
    let resource = create_test_resource(vec![("Version", "v1.2.3")]);
    let filter = TagFilter::new("Version".to_string(), TagFilterType::Regex)
        .with_pattern(r"^v\d+\.\d+\.\d+$".to_string());

    assert!(filter.matches(&resource));
}

#[test]
fn test_regex_filter_rejects_non_matching_pattern() {
    let resource = create_test_resource(vec![("Version", "1.2.3")]);
    let filter = TagFilter::new("Version".to_string(), TagFilterType::Regex)
        .with_pattern(r"^v\d+\.\d+\.\d+$".to_string());

    assert!(!filter.matches(&resource));
}

#[test]
fn test_regex_filter_handles_invalid_pattern() {
    let resource = create_test_resource(vec![("Version", "v1.2.3")]);
    let filter = TagFilter::new("Version".to_string(), TagFilterType::Regex)
        .with_pattern("[invalid".to_string()); // Invalid regex

    // Invalid regex should not match
    assert!(!filter.matches(&resource));
}

#[test]
fn test_regex_filter_rejects_missing_tag() {
    let resource = create_test_resource(vec![("Team", "Backend")]);
    let filter = TagFilter::new("Version".to_string(), TagFilterType::Regex)
        .with_pattern(r"^v\d+\.\d+\.\d+$".to_string());

    assert!(!filter.matches(&resource));
}

// ============================================================================
// TagFilter Validation Tests
// ============================================================================

#[test]
fn test_filter_validation_requires_tag_key() {
    let filter = TagFilter::new("".to_string(), TagFilterType::Equals)
        .with_values(vec!["Production".to_string()]);

    assert!(!filter.is_valid());
}

#[test]
fn test_filter_validation_equals_requires_values() {
    let filter = TagFilter::new("Environment".to_string(), TagFilterType::Equals);

    assert!(!filter.is_valid());
}

#[test]
fn test_filter_validation_exists_does_not_require_values() {
    let filter = TagFilter::new("Environment".to_string(), TagFilterType::Exists);

    assert!(filter.is_valid());
}

#[test]
fn test_filter_validation_not_exists_does_not_require_values() {
    let filter = TagFilter::new("Environment".to_string(), TagFilterType::NotExists);

    assert!(filter.is_valid());
}

#[test]
fn test_filter_validation_regex_requires_pattern() {
    let filter = TagFilter::new("Version".to_string(), TagFilterType::Regex);

    assert!(!filter.is_valid());
}

#[test]
fn test_filter_validation_regex_with_valid_pattern() {
    let filter = TagFilter::new("Version".to_string(), TagFilterType::Regex)
        .with_pattern(r"^v\d+".to_string());

    assert!(filter.is_valid());
}

// ============================================================================
// TagFilterGroup Tests - AND Operator
// ============================================================================

#[test]
fn test_filter_group_and_all_filters_must_match() {
    let resource = create_test_resource(vec![
        ("Environment", "Production"),
        ("Team", "Backend"),
    ]);

    let mut group = TagFilterGroup::new().with_operator(BooleanOperator::And);
    group.add_filter(
        TagFilter::new("Environment".to_string(), TagFilterType::Equals)
            .with_values(vec!["Production".to_string()]),
    );
    group.add_filter(
        TagFilter::new("Team".to_string(), TagFilterType::Equals)
            .with_values(vec!["Backend".to_string()]),
    );

    assert!(group.matches(&resource));
}

#[test]
fn test_filter_group_and_rejects_if_one_filter_fails() {
    let resource = create_test_resource(vec![
        ("Environment", "Production"),
        ("Team", "Frontend"),
    ]);

    let mut group = TagFilterGroup::new().with_operator(BooleanOperator::And);
    group.add_filter(
        TagFilter::new("Environment".to_string(), TagFilterType::Equals)
            .with_values(vec!["Production".to_string()]),
    );
    group.add_filter(
        TagFilter::new("Team".to_string(), TagFilterType::Equals)
            .with_values(vec!["Backend".to_string()]),
    );

    assert!(!group.matches(&resource));
}

// ============================================================================
// TagFilterGroup Tests - OR Operator
// ============================================================================

#[test]
fn test_filter_group_or_matches_if_one_filter_succeeds() {
    let resource = create_test_resource(vec![
        ("Environment", "Staging"),
        ("Team", "Backend"),
    ]);

    let mut group = TagFilterGroup::new().with_operator(BooleanOperator::Or);
    group.add_filter(
        TagFilter::new("Environment".to_string(), TagFilterType::Equals)
            .with_values(vec!["Production".to_string()]),
    );
    group.add_filter(
        TagFilter::new("Team".to_string(), TagFilterType::Equals)
            .with_values(vec!["Backend".to_string()]),
    );

    assert!(group.matches(&resource));
}

#[test]
fn test_filter_group_or_rejects_if_all_filters_fail() {
    let resource = create_test_resource(vec![
        ("Environment", "Staging"),
        ("Team", "Frontend"),
    ]);

    let mut group = TagFilterGroup::new().with_operator(BooleanOperator::Or);
    group.add_filter(
        TagFilter::new("Environment".to_string(), TagFilterType::Equals)
            .with_values(vec!["Production".to_string()]),
    );
    group.add_filter(
        TagFilter::new("Team".to_string(), TagFilterType::Equals)
            .with_values(vec!["Backend".to_string()]),
    );

    assert!(!group.matches(&resource));
}

// ============================================================================
// TagFilterGroup Tests - Empty Groups
// ============================================================================

#[test]
fn test_empty_filter_group_matches_all_resources() {
    let resource = create_test_resource(vec![("Environment", "Production")]);
    let group = TagFilterGroup::new();

    assert!(group.matches(&resource));
}

#[test]
fn test_empty_filter_group_is_valid() {
    let group = TagFilterGroup::new();

    assert!(group.is_valid());
}

// ============================================================================
// TagFilterGroup Tests - Nested Groups
// ============================================================================

#[test]
fn test_nested_filter_groups_complex_logic() {
    // Test: (Environment = Production OR Environment = Staging) AND Team = Backend
    let resource = create_test_resource(vec![
        ("Environment", "Staging"),
        ("Team", "Backend"),
    ]);

    // Create sub-group for Environment OR condition
    let mut env_group = TagFilterGroup::new().with_operator(BooleanOperator::Or);
    env_group.add_filter(
        TagFilter::new("Environment".to_string(), TagFilterType::Equals)
            .with_values(vec!["Production".to_string()]),
    );
    env_group.add_filter(
        TagFilter::new("Environment".to_string(), TagFilterType::Equals)
            .with_values(vec!["Staging".to_string()]),
    );

    // Create main group with AND operator
    let mut main_group = TagFilterGroup::new().with_operator(BooleanOperator::And);
    main_group.add_sub_group(env_group);
    main_group.add_filter(
        TagFilter::new("Team".to_string(), TagFilterType::Equals)
            .with_values(vec!["Backend".to_string()]),
    );

    assert!(main_group.matches(&resource));
}

#[test]
fn test_nested_filter_groups_rejects_partial_match() {
    // Test: (Environment = Production OR Environment = Staging) AND Team = Backend
    let resource = create_test_resource(vec![
        ("Environment", "Staging"),
        ("Team", "Frontend"), // This doesn't match
    ]);

    // Create sub-group for Environment OR condition
    let mut env_group = TagFilterGroup::new().with_operator(BooleanOperator::Or);
    env_group.add_filter(
        TagFilter::new("Environment".to_string(), TagFilterType::Equals)
            .with_values(vec!["Production".to_string()]),
    );
    env_group.add_filter(
        TagFilter::new("Environment".to_string(), TagFilterType::Equals)
            .with_values(vec!["Staging".to_string()]),
    );

    // Create main group with AND operator
    let mut main_group = TagFilterGroup::new().with_operator(BooleanOperator::And);
    main_group.add_sub_group(env_group);
    main_group.add_filter(
        TagFilter::new("Team".to_string(), TagFilterType::Equals)
            .with_values(vec!["Backend".to_string()]),
    );

    assert!(!main_group.matches(&resource));
}

// ============================================================================
// TagFilterGroup Helper Methods Tests
// ============================================================================

#[test]
fn test_filter_group_count_includes_nested_filters() {
    let mut main_group = TagFilterGroup::new();
    main_group.add_filter(TagFilter::new("Environment".to_string(), TagFilterType::Exists));

    let mut sub_group = TagFilterGroup::new();
    sub_group.add_filter(TagFilter::new("Team".to_string(), TagFilterType::Exists));
    sub_group.add_filter(TagFilter::new("Project".to_string(), TagFilterType::Exists));

    main_group.add_sub_group(sub_group);

    assert_eq!(main_group.filter_count(), 3);
}

#[test]
fn test_collect_filter_tag_keys_includes_all_keys() {
    let mut main_group = TagFilterGroup::new();
    main_group.add_filter(TagFilter::new("Environment".to_string(), TagFilterType::Exists));

    let mut sub_group = TagFilterGroup::new();
    sub_group.add_filter(TagFilter::new("Team".to_string(), TagFilterType::Exists));
    sub_group.add_filter(TagFilter::new("Project".to_string(), TagFilterType::Exists));

    main_group.add_sub_group(sub_group);

    let mut tag_keys = Vec::new();
    main_group.collect_filter_tag_keys(&mut tag_keys);

    assert_eq!(tag_keys.len(), 3);
    assert!(tag_keys.contains(&"Environment".to_string()));
    assert!(tag_keys.contains(&"Team".to_string()));
    assert!(tag_keys.contains(&"Project".to_string()));
}

#[test]
fn test_collect_filter_tag_keys_avoids_duplicates() {
    let mut main_group = TagFilterGroup::new();
    main_group.add_filter(TagFilter::new("Environment".to_string(), TagFilterType::Exists));
    main_group.add_filter(
        TagFilter::new("Environment".to_string(), TagFilterType::Equals)
            .with_values(vec!["Production".to_string()]),
    );

    let mut tag_keys = Vec::new();
    main_group.collect_filter_tag_keys(&mut tag_keys);

    assert_eq!(tag_keys.len(), 1);
    assert_eq!(tag_keys[0], "Environment");
}

// ============================================================================
// Edge Cases and Complex Scenarios
// ============================================================================

#[test]
fn test_multiple_tags_with_same_key_uses_first_value() {
    // While AWS doesn't typically allow duplicate tag keys,
    // we should handle it gracefully if it occurs in our data structure
    let mut resource = create_test_resource(vec![]);
    resource.tags.push(ResourceTag {
        key: "Environment".to_string(),
        value: "Production".to_string(),
    });
    resource.tags.push(ResourceTag {
        key: "Environment".to_string(),
        value: "Staging".to_string(),
    });

    let filter = TagFilter::new("Environment".to_string(), TagFilterType::Equals)
        .with_values(vec!["Production".to_string()]);

    // Should match using the first occurrence
    assert!(filter.matches(&resource));
}

#[test]
fn test_filter_with_empty_values_list_is_invalid() {
    let filter = TagFilter::new("Environment".to_string(), TagFilterType::Equals);

    assert!(!filter.is_valid());
}

#[test]
fn test_case_sensitive_tag_key_matching() {
    let resource = create_test_resource(vec![("environment", "Production")]);
    let filter = TagFilter::new("Environment".to_string(), TagFilterType::Equals)
        .with_values(vec!["Production".to_string()]);

    // Tag keys are case-sensitive
    assert!(!filter.matches(&resource));
}

#[test]
fn test_complex_real_world_scenario() {
    // Real-world scenario: Find production backend resources that are not archived
    let resource = create_test_resource(vec![
        ("Environment", "Production"),
        ("Team", "Backend"),
        ("Application", "api-gateway"),
        ("Status", "active"),
    ]);

    let mut main_group = TagFilterGroup::new().with_operator(BooleanOperator::And);

    // Environment must be Production
    main_group.add_filter(
        TagFilter::new("Environment".to_string(), TagFilterType::Equals)
            .with_values(vec!["Production".to_string()]),
    );

    // Team must be Backend
    main_group.add_filter(
        TagFilter::new("Team".to_string(), TagFilterType::Equals)
            .with_values(vec!["Backend".to_string()]),
    );

    // Status must not be archived
    main_group.add_filter(
        TagFilter::new("Status".to_string(), TagFilterType::NotEquals)
            .with_values(vec!["archived".to_string()]),
    );

    // Application must contain "api"
    main_group.add_filter(
        TagFilter::new("Application".to_string(), TagFilterType::Contains)
            .with_values(vec!["api".to_string()]),
    );

    assert!(main_group.matches(&resource));
}
