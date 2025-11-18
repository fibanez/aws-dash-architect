//! Tag Grouping System Unit Tests
//!
//! Comprehensive unit tests for the tag-based grouping functionality, covering both
//! single-tag grouping and hierarchical multi-tag grouping modes.
//!
//! # Test Coverage
//!
//! - **GroupingMode Variants**: Tests for all grouping modes (Account, Region, Type, Tag, TagHierarchy)
//! - **Validation**: Ensures grouping modes are properly configured
//! - **Display Names**: Verifies correct UI labels for grouping modes
//! - **Tag Keys Extraction**: Tests extraction of tag keys from grouping modes
//! - **Edge Cases**: Empty hierarchies, long tag names, invalid configurations
//!
//! # Grouping Modes Tested
//!
//! 1. ByAccount - Group resources by AWS account
//! 2. ByRegion - Group resources by AWS region
//! 3. ByResourceType - Group resources by AWS resource type (e.g., AWS::EC2::Instance)
//! 4. ByTag - Group resources by a single tag key
//! 5. ByTagHierarchy - Group resources by multiple tag keys in hierarchical order

use awsdash::app::resource_explorer::state::GroupingMode;

// ============================================================================
// GroupingMode - Default Modes Tests
// ============================================================================

#[test]
fn test_grouping_mode_by_account_display_name() {
    let mode = GroupingMode::ByAccount;
    assert_eq!(mode.display_name(), "Account");
}

#[test]
fn test_grouping_mode_by_region_display_name() {
    let mode = GroupingMode::ByRegion;
    assert_eq!(mode.display_name(), "Region");
}

#[test]
fn test_grouping_mode_by_resource_type_display_name() {
    let mode = GroupingMode::ByResourceType;
    assert_eq!(mode.display_name(), "Resource Type");
}

#[test]
fn test_default_modes_includes_all_standard_modes() {
    let modes = GroupingMode::default_modes();
    assert_eq!(modes.len(), 3);
    assert!(matches!(modes[0], GroupingMode::ByAccount));
    assert!(matches!(modes[1], GroupingMode::ByRegion));
    assert!(matches!(modes[2], GroupingMode::ByResourceType));
}

// ============================================================================
// GroupingMode - ByTag Tests
// ============================================================================

#[test]
fn test_by_tag_grouping_display_name() {
    let mode = GroupingMode::ByTag("Environment".to_string());
    assert_eq!(mode.display_name(), "Tag: Environment");
}

#[test]
fn test_by_tag_grouping_short_label() {
    let mode = GroupingMode::ByTag("Environment".to_string());
    assert_eq!(mode.short_label(), "Environment");
}

#[test]
fn test_by_tag_grouping_long_name_short_label() {
    let mode = GroupingMode::ByTag("VeryLongTagKeyName".to_string());
    let short = mode.short_label();
    // Should truncate to 12 chars + "..."
    assert_eq!(short, "VeryLongTagK...");
}

#[test]
fn test_by_tag_grouping_is_tag_based() {
    let mode = GroupingMode::ByTag("Environment".to_string());
    assert!(mode.is_tag_based());
}

#[test]
fn test_by_tag_grouping_tag_keys() {
    let mode = GroupingMode::ByTag("Environment".to_string());
    let keys = mode.tag_keys();
    assert_eq!(keys.len(), 1);
    assert_eq!(keys[0], "Environment");
}

#[test]
fn test_by_tag_grouping_validation_valid() {
    let mode = GroupingMode::ByTag("Environment".to_string());
    assert!(mode.is_valid());
}

#[test]
fn test_by_tag_grouping_validation_empty_key() {
    let mode = GroupingMode::ByTag("".to_string());
    assert!(!mode.is_valid());
}

// ============================================================================
// GroupingMode - ByTagHierarchy Tests
// ============================================================================

#[test]
fn test_by_tag_hierarchy_single_key_display_name() {
    let mode = GroupingMode::ByTagHierarchy(vec!["Environment".to_string()]);
    assert_eq!(mode.display_name(), "Tag: Environment");
}

#[test]
fn test_by_tag_hierarchy_multiple_keys_display_name() {
    let mode = GroupingMode::ByTagHierarchy(vec![
        "Environment".to_string(),
        "Team".to_string(),
        "Project".to_string(),
    ]);
    assert_eq!(mode.display_name(), "Tag Hierarchy: Environment > ...");
}

#[test]
fn test_by_tag_hierarchy_empty_display_name() {
    let mode = GroupingMode::ByTagHierarchy(vec![]);
    assert_eq!(mode.display_name(), "Tag Hierarchy (empty)");
}

#[test]
fn test_by_tag_hierarchy_short_label_single_key() {
    let mode = GroupingMode::ByTagHierarchy(vec!["Environment".to_string()]);
    assert_eq!(mode.short_label(), "Environment");
}

#[test]
fn test_by_tag_hierarchy_short_label_multiple_keys() {
    let mode = GroupingMode::ByTagHierarchy(vec![
        "Environment".to_string(),
        "Team".to_string(),
        "Project".to_string(),
    ]);
    // Should show first key + count
    assert_eq!(mode.short_label(), "Environment+2");
}

#[test]
fn test_by_tag_hierarchy_short_label_long_first_key() {
    let mode = GroupingMode::ByTagHierarchy(vec![
        "VeryLongTagKeyName".to_string(),
        "Team".to_string(),
    ]);
    // Should truncate first key
    assert_eq!(mode.short_label(), "VeryLongT...+1");
}

#[test]
fn test_by_tag_hierarchy_is_tag_based() {
    let mode = GroupingMode::ByTagHierarchy(vec!["Environment".to_string(), "Team".to_string()]);
    assert!(mode.is_tag_based());
}

#[test]
fn test_by_tag_hierarchy_tag_keys() {
    let mode = GroupingMode::ByTagHierarchy(vec![
        "Environment".to_string(),
        "Team".to_string(),
        "Project".to_string(),
    ]);
    let keys = mode.tag_keys();
    assert_eq!(keys.len(), 3);
    assert_eq!(keys[0], "Environment");
    assert_eq!(keys[1], "Team");
    assert_eq!(keys[2], "Project");
}

#[test]
fn test_by_tag_hierarchy_validation_valid() {
    let mode = GroupingMode::ByTagHierarchy(vec![
        "Environment".to_string(),
        "Team".to_string(),
    ]);
    assert!(mode.is_valid());
}

#[test]
fn test_by_tag_hierarchy_validation_empty_list() {
    let mode = GroupingMode::ByTagHierarchy(vec![]);
    assert!(!mode.is_valid());
}

#[test]
fn test_by_tag_hierarchy_validation_empty_key_in_list() {
    let mode = GroupingMode::ByTagHierarchy(vec![
        "Environment".to_string(),
        "".to_string(), // Empty key
        "Team".to_string(),
    ]);
    assert!(!mode.is_valid());
}

// ============================================================================
// GroupingMode - Default Modes Not Tag-Based
// ============================================================================

#[test]
fn test_by_account_is_not_tag_based() {
    let mode = GroupingMode::ByAccount;
    assert!(!mode.is_tag_based());
}

#[test]
fn test_by_region_is_not_tag_based() {
    let mode = GroupingMode::ByRegion;
    assert!(!mode.is_tag_based());
}

#[test]
fn test_by_resource_type_is_not_tag_based() {
    let mode = GroupingMode::ByResourceType;
    assert!(!mode.is_tag_based());
}

#[test]
fn test_default_modes_tag_keys_empty() {
    let mode = GroupingMode::ByAccount;
    assert!(mode.tag_keys().is_empty());

    let mode = GroupingMode::ByRegion;
    assert!(mode.tag_keys().is_empty());

    let mode = GroupingMode::ByResourceType;
    assert!(mode.tag_keys().is_empty());
}

#[test]
fn test_default_modes_all_valid() {
    assert!(GroupingMode::ByAccount.is_valid());
    assert!(GroupingMode::ByRegion.is_valid());
    assert!(GroupingMode::ByResourceType.is_valid());
}

// ============================================================================
// GroupingMode - Short Label Tests
// ============================================================================

#[test]
fn test_short_label_account() {
    let mode = GroupingMode::ByAccount;
    assert_eq!(mode.short_label(), "Account");
}

#[test]
fn test_short_label_region() {
    let mode = GroupingMode::ByRegion;
    assert_eq!(mode.short_label(), "Region");
}

#[test]
fn test_short_label_resource_type() {
    let mode = GroupingMode::ByResourceType;
    assert_eq!(mode.short_label(), "Type");
}

#[test]
fn test_short_label_tag_hierarchy_empty() {
    let mode = GroupingMode::ByTagHierarchy(vec![]);
    assert_eq!(mode.short_label(), "Hierarchy");
}

// ============================================================================
// Real-World Scenarios
// ============================================================================

#[test]
fn test_real_world_environment_team_project_hierarchy() {
    let mode = GroupingMode::ByTagHierarchy(vec![
        "Environment".to_string(),
        "Team".to_string(),
        "Project".to_string(),
    ]);

    assert!(mode.is_valid());
    assert!(mode.is_tag_based());
    assert_eq!(mode.tag_keys().len(), 3);
    assert_eq!(mode.display_name(), "Tag Hierarchy: Environment > ...");
    assert_eq!(mode.short_label(), "Environment+2");
}

#[test]
fn test_real_world_cost_center_grouping() {
    let mode = GroupingMode::ByTag("CostCenter".to_string());

    assert!(mode.is_valid());
    assert!(mode.is_tag_based());
    assert_eq!(mode.tag_keys().len(), 1);
    assert_eq!(mode.display_name(), "Tag: CostCenter");
    assert_eq!(mode.short_label(), "CostCenter");
}

#[test]
fn test_real_world_compliance_classification_hierarchy() {
    let mode = GroupingMode::ByTagHierarchy(vec![
        "ComplianceLevel".to_string(),
        "DataClassification".to_string(),
    ]);

    assert!(mode.is_valid());
    assert!(mode.is_tag_based());
    assert_eq!(mode.tag_keys().len(), 2);
    assert_eq!(mode.display_name(), "Tag Hierarchy: ComplianceLevel > ...");
    // "ComplianceLevel" is 15 chars (> 12), so truncates to 9 chars + "..."
    assert_eq!(mode.short_label(), "Complianc...+1");
}

#[test]
fn test_switching_between_grouping_modes() {
    // Simulate UI switching between different grouping modes
    let mut current_mode = GroupingMode::ByAccount;
    assert_eq!(current_mode.display_name(), "Account");

    current_mode = GroupingMode::ByTag("Environment".to_string());
    assert_eq!(current_mode.display_name(), "Tag: Environment");

    current_mode = GroupingMode::ByTagHierarchy(vec![
        "Environment".to_string(),
        "Team".to_string(),
    ]);
    assert_eq!(current_mode.display_name(), "Tag Hierarchy: Environment > ...");

    current_mode = GroupingMode::ByRegion;
    assert_eq!(current_mode.display_name(), "Region");
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_tag_hierarchy_with_single_character_keys() {
    let mode = GroupingMode::ByTagHierarchy(vec!["A".to_string(), "B".to_string(), "C".to_string()]);

    assert!(mode.is_valid());
    assert_eq!(mode.short_label(), "A+2");
}

#[test]
fn test_tag_hierarchy_with_unicode_characters() {
    let mode = GroupingMode::ByTagHierarchy(vec!["环境".to_string(), "团队".to_string()]);

    assert!(mode.is_valid());
    assert_eq!(mode.tag_keys().len(), 2);
}

#[test]
fn test_tag_with_special_characters() {
    let mode = GroupingMode::ByTag("aws:cloudformation:stack-name".to_string());

    assert!(mode.is_valid());
    assert_eq!(mode.display_name(), "Tag: aws:cloudformation:stack-name");
}

#[test]
fn test_tag_hierarchy_preserves_order() {
    let keys = vec![
        "Environment".to_string(),
        "Team".to_string(),
        "Project".to_string(),
    ];
    let mode = GroupingMode::ByTagHierarchy(keys.clone());

    let extracted_keys = mode.tag_keys();
    assert_eq!(extracted_keys, keys);
}

#[test]
fn test_multiple_hierarchy_modes_with_same_keys_different_order() {
    let mode1 = GroupingMode::ByTagHierarchy(vec![
        "Environment".to_string(),
        "Team".to_string(),
    ]);
    let mode2 = GroupingMode::ByTagHierarchy(vec![
        "Team".to_string(),
        "Environment".to_string(),
    ]);

    // Both are valid but represent different grouping hierarchies
    assert!(mode1.is_valid());
    assert!(mode2.is_valid());
    assert_ne!(mode1.tag_keys(), mode2.tag_keys());
}

#[test]
fn test_tag_name_exactly_15_characters() {
    // Test boundary condition for short_label truncation (>15 triggers truncation)
    let mode = GroupingMode::ByTag("ExactlyFifteen!".to_string());
    assert_eq!(mode.short_label(), "ExactlyFifteen!");
}

#[test]
fn test_tag_name_16_characters_truncates() {
    // Test boundary condition for short_label truncation (>15 triggers truncation)
    let mode = GroupingMode::ByTag("SixteenCharacter".to_string());
    assert_eq!(mode.short_label(), "SixteenChara...");
}

#[test]
fn test_all_modes_returns_only_default_modes() {
    let modes = GroupingMode::all_modes();
    assert_eq!(modes.len(), 3);
    assert!(matches!(modes[0], GroupingMode::ByAccount));
    assert!(matches!(modes[1], GroupingMode::ByRegion));
    assert!(matches!(modes[2], GroupingMode::ByResourceType));
}
