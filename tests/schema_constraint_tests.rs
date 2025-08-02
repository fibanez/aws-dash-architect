//! Unit tests for schema constraint parsing functionality
//!
//! These tests validate the enhanced PropertyDefinition structure and
//! schema constraint parsing capabilities for CloudFormation property validation.

use awsdash::app::cfn_resources::{
    load_property_definitions_with_constraints, parse_schema_constraints, PropertyDefinition,
    SchemaConstraints,
};
use serde_json::json;

#[test]
fn test_parse_enum_constraints() {
    let schema = json!({
        "type": "string",
        "enum": ["Private", "PublicRead", "PublicReadWrite"]
    });

    let constraints = parse_schema_constraints(&schema);

    assert!(constraints.enum_values.is_some());
    let enums = constraints.enum_values.unwrap();
    assert_eq!(enums.len(), 3);
    assert!(enums.contains(&"Private".to_string()));
    assert!(enums.contains(&"PublicRead".to_string()));
    assert!(enums.contains(&"PublicReadWrite".to_string()));

    assert!(constraints.pattern.is_none());
    assert!(constraints.min_length.is_none());
    assert!(constraints.max_length.is_none());
    assert!(constraints.min_value.is_none());
    assert!(constraints.max_value.is_none());
    assert!(constraints.unique_items.is_none());
}

#[test]
fn test_parse_string_pattern_constraints() {
    let schema = json!({
        "type": "string",
        "pattern": "^[a-zA-Z0-9-]+$",
        "minLength": 3,
        "maxLength": 63
    });

    let constraints = parse_schema_constraints(&schema);

    assert!(constraints.enum_values.is_none());
    assert_eq!(constraints.pattern, Some("^[a-zA-Z0-9-]+$".to_string()));
    assert_eq!(constraints.min_length, Some(3));
    assert_eq!(constraints.max_length, Some(63));
    assert!(constraints.min_value.is_none());
    assert!(constraints.max_value.is_none());
    assert!(constraints.unique_items.is_none());
}

#[test]
fn test_parse_numeric_constraints() {
    let schema = json!({
        "type": "number",
        "minimum": 1.0,
        "maximum": 100.0
    });

    let constraints = parse_schema_constraints(&schema);

    assert!(constraints.enum_values.is_none());
    assert!(constraints.pattern.is_none());
    assert!(constraints.min_length.is_none());
    assert!(constraints.max_length.is_none());
    assert_eq!(constraints.min_value, Some(1.0));
    assert_eq!(constraints.max_value, Some(100.0));
    assert!(constraints.unique_items.is_none());
}

#[test]
fn test_parse_array_constraints() {
    let schema = json!({
        "type": "array",
        "uniqueItems": true,
        "minLength": 1,
        "maxLength": 10
    });

    let constraints = parse_schema_constraints(&schema);

    assert!(constraints.enum_values.is_none());
    assert!(constraints.pattern.is_none());
    assert_eq!(constraints.min_length, Some(1));
    assert_eq!(constraints.max_length, Some(10));
    assert!(constraints.min_value.is_none());
    assert!(constraints.max_value.is_none());
    assert_eq!(constraints.unique_items, Some(true));
}

#[test]
fn test_parse_complex_schema_with_multiple_constraints() {
    let schema = json!({
        "type": "string",
        "enum": ["t2.micro", "t2.small", "t2.medium"],
        "pattern": "^t2\\.(micro|small|medium|large)$",
        "minLength": 7,
        "maxLength": 15
    });

    let constraints = parse_schema_constraints(&schema);

    // Should parse all applicable constraints
    assert!(constraints.enum_values.is_some());
    let enums = constraints.enum_values.unwrap();
    assert_eq!(enums.len(), 3);
    assert!(enums.contains(&"t2.micro".to_string()));

    assert_eq!(
        constraints.pattern,
        Some("^t2\\.(micro|small|medium|large)$".to_string())
    );
    assert_eq!(constraints.min_length, Some(7));
    assert_eq!(constraints.max_length, Some(15));
    assert!(constraints.min_value.is_none());
    assert!(constraints.max_value.is_none());
    assert!(constraints.unique_items.is_none());
}

#[test]
fn test_parse_empty_schema() {
    let schema = json!({
        "type": "string"
    });

    let constraints = parse_schema_constraints(&schema);

    // Should return None for all constraints when not present
    assert!(constraints.enum_values.is_none());
    assert!(constraints.pattern.is_none());
    assert!(constraints.min_length.is_none());
    assert!(constraints.max_length.is_none());
    assert!(constraints.min_value.is_none());
    assert!(constraints.max_value.is_none());
    assert!(constraints.unique_items.is_none());
}

#[test]
fn test_parse_invalid_constraint_values() {
    let schema = json!({
        "type": "string",
        "enum": "not-an-array",
        "pattern": 123,
        "minLength": "not-a-number",
        "maximum": "not-a-number"
    });

    let constraints = parse_schema_constraints(&schema);

    // Should gracefully handle invalid values by returning None
    assert!(constraints.enum_values.is_none());
    assert!(constraints.pattern.is_none());
    assert!(constraints.min_length.is_none());
    assert!(constraints.max_length.is_none());
    assert!(constraints.min_value.is_none());
    assert!(constraints.max_value.is_none());
    assert!(constraints.unique_items.is_none());
}

#[test]
fn test_enhanced_property_definition_structure() {
    // Test that PropertyDefinition can hold all the new constraint fields
    let property_def = PropertyDefinition {
        documentation: "Test property".to_string(),
        required: true,
        primitive_type: Some("String".to_string()),
        type_name: None,
        item_type: None,
        update_type: "Mutable".to_string(),
        enum_values: Some(vec!["Option1".to_string(), "Option2".to_string()]),
        pattern: Some("^[a-z]+$".to_string()),
        min_length: Some(1),
        max_length: Some(50),
        min_value: Some(0.0),
        max_value: Some(100.0),
        unique_items: Some(true),
    };

    // Verify all fields are accessible
    assert_eq!(property_def.documentation, "Test property");
    assert!(property_def.required);
    assert_eq!(property_def.primitive_type, Some("String".to_string()));

    // Verify enhanced constraint fields
    assert!(property_def.enum_values.is_some());
    assert_eq!(property_def.enum_values.unwrap().len(), 2);
    assert_eq!(property_def.pattern, Some("^[a-z]+$".to_string()));
    assert_eq!(property_def.min_length, Some(1));
    assert_eq!(property_def.max_length, Some(50));
    assert_eq!(property_def.min_value, Some(0.0));
    assert_eq!(property_def.max_value, Some(100.0));
    assert_eq!(property_def.unique_items, Some(true));
}

#[test]
fn test_property_definition_with_no_constraints() {
    // Test PropertyDefinition with no enhanced constraints (backward compatibility)
    let property_def = PropertyDefinition {
        documentation: "Simple property".to_string(),
        required: false,
        primitive_type: Some("String".to_string()),
        type_name: None,
        item_type: None,
        update_type: "Immutable".to_string(),
        enum_values: None,
        pattern: None,
        min_length: None,
        max_length: None,
        min_value: None,
        max_value: None,
        unique_items: None,
    };

    // Verify backward compatibility - should work without constraints
    assert_eq!(property_def.documentation, "Simple property");
    assert!(!property_def.required);
    assert!(property_def.enum_values.is_none());
    assert!(property_def.pattern.is_none());
    assert!(property_def.min_length.is_none());
    assert!(property_def.max_length.is_none());
    assert!(property_def.min_value.is_none());
    assert!(property_def.max_value.is_none());
    assert!(property_def.unique_items.is_none());
}

#[test]
fn test_schema_constraints_structure() {
    // Test that SchemaConstraints can hold all constraint types
    let constraints = SchemaConstraints {
        enum_values: Some(vec!["value1".to_string(), "value2".to_string()]),
        pattern: Some("^[a-z]+$".to_string()),
        min_length: Some(5),
        max_length: Some(50),
        min_value: Some(0.0),
        max_value: Some(100.0),
        unique_items: Some(true),
    };

    // Verify all fields are accessible
    assert!(constraints.enum_values.is_some());
    assert_eq!(constraints.enum_values.unwrap().len(), 2);
    assert_eq!(constraints.pattern, Some("^[a-z]+$".to_string()));
    assert_eq!(constraints.min_length, Some(5));
    assert_eq!(constraints.max_length, Some(50));
    assert_eq!(constraints.min_value, Some(0.0));
    assert_eq!(constraints.max_value, Some(100.0));
    assert_eq!(constraints.unique_items, Some(true));
}

#[test]
fn test_enhanced_property_definition_loading() {
    // Test that load_property_definitions_with_constraints works without errors
    // This will use fallback behavior since we don't have actual schema files in tests

    // This should not crash and should return at least some result (even if empty)
    let result = load_property_definitions_with_constraints("us-east-1", "AWS::S3::Bucket");

    // Should either succeed or fail gracefully - the point is it doesn't crash
    match result {
        Ok(_properties) => {
            // Success case - enhanced properties loaded
        }
        Err(_) => {
            // Expected failure case - no schema files available in test environment
        }
    }
}

#[test]
fn test_schema_constraints_default_values() {
    // Test SchemaConstraints with default (None) values
    let constraints = SchemaConstraints {
        enum_values: None,
        pattern: None,
        min_length: None,
        max_length: None,
        min_value: None,
        max_value: None,
        unique_items: None,
    };

    // Verify all fields are None by default
    assert!(constraints.enum_values.is_none());
    assert!(constraints.pattern.is_none());
    assert!(constraints.min_length.is_none());
    assert!(constraints.max_length.is_none());
    assert!(constraints.min_value.is_none());
    assert!(constraints.max_value.is_none());
    assert!(constraints.unique_items.is_none());
}

#[test]
fn test_constraint_parsing_integration() {
    // Test the integration between parse_schema_constraints and SchemaConstraints
    let complex_schema = json!({
        "type": "object",
        "properties": {
            "EnumProperty": {
                "type": "string",
                "enum": ["option1", "option2", "option3"]
            },
            "PatternProperty": {
                "type": "string",
                "pattern": "^[A-Z][a-z]+$",
                "minLength": 2,
                "maxLength": 20
            },
            "ArrayProperty": {
                "type": "array",
                "uniqueItems": true,
                "minLength": 1,
                "maxLength": 10
            }
        }
    });

    // Test each property type
    let enum_prop = complex_schema["properties"]["EnumProperty"].clone();
    let enum_constraints = parse_schema_constraints(&enum_prop);

    assert!(enum_constraints.enum_values.is_some());
    assert_eq!(enum_constraints.enum_values.unwrap().len(), 3);
    assert!(enum_constraints.pattern.is_none());

    // Test pattern property
    let pattern_prop = complex_schema["properties"]["PatternProperty"].clone();
    let pattern_constraints = parse_schema_constraints(&pattern_prop);

    assert!(pattern_constraints.enum_values.is_none());
    assert_eq!(
        pattern_constraints.pattern,
        Some("^[A-Z][a-z]+$".to_string())
    );
    assert_eq!(pattern_constraints.min_length, Some(2));
    assert_eq!(pattern_constraints.max_length, Some(20));
}
