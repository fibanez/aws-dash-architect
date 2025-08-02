//! Tests for CloudFormation intrinsic function classification and property editing interface selection
//!
//! This test suite verifies that the new intrinsic function detection system correctly
//! identifies CloudFormation intrinsic functions and routes them to the appropriate
//! editing interface (Value Editor, Reference Picker, or JSON Editor).

use awsdash::app::cfn_intrinsic_functions::{
    classify_property_value, detect_intrinsic_function, detect_intrinsic_function_in_string,
    IntrinsicFunctionType, PreferredEditor, PropertyValueClassification,
};
use serde_json::json;

#[test]
fn test_comprehensive_intrinsic_function_detection() {
    // Test all major CloudFormation intrinsic functions

    // Simple references
    let ref_value = json!({"Ref": "MyResource"});
    assert_eq!(
        detect_intrinsic_function(&ref_value),
        Some(IntrinsicFunctionType::Ref)
    );

    let getatt_value = json!({"Fn::GetAtt": ["MyResource", "Arn"]});
    assert_eq!(
        detect_intrinsic_function(&getatt_value),
        Some(IntrinsicFunctionType::GetAtt)
    );

    let import_value = json!({"Fn::ImportValue": "SharedVPC"});
    assert_eq!(
        detect_intrinsic_function(&import_value),
        Some(IntrinsicFunctionType::ImportValue)
    );

    // Complex functions
    let sub_value = json!({"Fn::Sub": ["Hello ${param}", {"param": {"Ref": "MyParam"}}]});
    assert_eq!(
        detect_intrinsic_function(&sub_value),
        Some(IntrinsicFunctionType::Sub)
    );

    let join_value = json!({"Fn::Join": [",", ["a", "b", "c"]]});
    assert_eq!(
        detect_intrinsic_function(&join_value),
        Some(IntrinsicFunctionType::Join)
    );

    let select_value = json!({"Fn::Select": [0, {"Fn::GetAZs": ""}]});
    assert_eq!(
        detect_intrinsic_function(&select_value),
        Some(IntrinsicFunctionType::Select)
    );

    // Condition functions
    let if_value = json!({"Fn::If": ["IsProduction", "t3.large", "t3.micro"]});
    assert_eq!(
        detect_intrinsic_function(&if_value),
        Some(IntrinsicFunctionType::If)
    );

    let and_value = json!({"Fn::And": [{"Fn::Equals": ["${Environment}", "prod"]}, {"Fn::Not": [{"Fn::Equals": ["${Debug}", "true"]}]}]});
    assert_eq!(
        detect_intrinsic_function(&and_value),
        Some(IntrinsicFunctionType::And)
    );
}

#[test]
fn test_property_value_classification_for_editing_interface() {
    // Test that different types of values are classified correctly for the editing interface

    // Literal values should use Value Editor
    let literal_string = json!("simple string");
    let classification = classify_property_value(&literal_string);
    assert_eq!(classification, PropertyValueClassification::LiteralValue);
    assert_eq!(
        classification.preferred_editor(),
        PreferredEditor::ValueEditor
    );

    let literal_number = json!(42);
    let classification = classify_property_value(&literal_number);
    assert_eq!(classification, PropertyValueClassification::LiteralValue);
    assert_eq!(
        classification.preferred_editor(),
        PreferredEditor::ValueEditor
    );

    // Simple references should use Reference Picker
    let simple_ref = json!({"Ref": "MyResource"});
    let classification = classify_property_value(&simple_ref);
    assert_eq!(
        classification,
        PropertyValueClassification::SimpleReference(IntrinsicFunctionType::Ref)
    );
    assert_eq!(
        classification.preferred_editor(),
        PreferredEditor::ReferencePicker
    );

    let getatt_ref = json!({"Fn::GetAtt": ["MyResource", "Arn"]});
    let classification = classify_property_value(&getatt_ref);
    assert_eq!(
        classification,
        PropertyValueClassification::SimpleReference(IntrinsicFunctionType::GetAtt)
    );
    assert_eq!(
        classification.preferred_editor(),
        PreferredEditor::ReferencePicker
    );

    // Complex references should use JSON Editor
    let complex_sub =
        json!({"Fn::Sub": ["Hello ${AWS::Region} and ${param}", {"param": {"Ref": "MyParam"}}]});
    let classification = classify_property_value(&complex_sub);
    assert_eq!(
        classification,
        PropertyValueClassification::ComplexReference(IntrinsicFunctionType::Sub)
    );
    assert_eq!(
        classification.preferred_editor(),
        PreferredEditor::JsonEditor
    );

    let complex_join = json!({"Fn::Join": [",", [{"Ref": "Param1"}, {"Fn::GetAtt": ["Resource1", "Value"]}, "literal"]]});
    let classification = classify_property_value(&complex_join);
    assert_eq!(
        classification,
        PropertyValueClassification::ComplexReference(IntrinsicFunctionType::Join)
    );
    assert_eq!(
        classification.preferred_editor(),
        PreferredEditor::JsonEditor
    );

    // Strings with template syntax should use Value Editor
    let template_string = json!("Hello ${AWS::Region}");
    let classification = classify_property_value(&template_string);
    assert_eq!(
        classification,
        PropertyValueClassification::StringWithReferences
    );
    assert_eq!(
        classification.preferred_editor(),
        PreferredEditor::ValueEditor
    );
}

#[test]
fn test_string_intrinsic_function_detection() {
    // Test detection of intrinsic function syntax in strings

    // CloudFormation substitution syntax
    assert!(detect_intrinsic_function_in_string("Hello ${AWS::Region}"));
    assert!(detect_intrinsic_function_in_string(
        "arn:aws:s3:::${BucketName}/*"
    ));

    // YAML-style function calls (less common but possible)
    assert!(detect_intrinsic_function_in_string("!Ref MyResource"));
    assert!(detect_intrinsic_function_in_string(
        "!GetAtt MyResource.Arn"
    ));
    assert!(detect_intrinsic_function_in_string(
        "!Sub Hello ${AWS::Region}"
    ));

    // Strings without intrinsic functions
    assert!(!detect_intrinsic_function_in_string("simple string"));
    assert!(!detect_intrinsic_function_in_string(
        "just some text with $ but no {"
    ));
    assert!(!detect_intrinsic_function_in_string(
        "text with {braces} but no $"
    ));
}

#[test]
fn test_complex_nested_structures() {
    // Test arrays containing references
    let array_with_refs = json!([
        "literal value",
        {"Ref": "MyResource"},
        {"Fn::GetAtt": ["OtherResource", "Property"]}
    ]);
    let classification = classify_property_value(&array_with_refs);
    match classification {
        PropertyValueClassification::ComplexReference(_) => {
            assert_eq!(
                classification.preferred_editor(),
                PreferredEditor::JsonEditor
            );
        }
        _ => panic!("Expected ComplexReference for array with intrinsic functions"),
    }

    // Test objects containing references
    let object_with_refs = json!({
        "SimpleProperty": "literal value",
        "ResourceRef": {"Ref": "MyResource"},
        "ResourceArn": {"Fn::GetAtt": ["MyResource", "Arn"]}
    });
    let classification = classify_property_value(&object_with_refs);
    match classification {
        PropertyValueClassification::ComplexReference(_) => {
            assert_eq!(
                classification.preferred_editor(),
                PreferredEditor::JsonEditor
            );
        }
        _ => panic!("Expected ComplexReference for object with intrinsic functions"),
    }
}

#[test]
fn test_display_preview_formatting() {
    // Test that display previews are formatted correctly for different value types

    // Simple reference preview
    let ref_value = json!({"Ref": "MyBucket"});
    let classification = classify_property_value(&ref_value);
    let preview = classification.get_display_preview(&ref_value);
    assert_eq!(preview, "!Ref MyBucket");

    // GetAtt reference preview
    let getatt_value = json!({"Fn::GetAtt": ["MyBucket", "Arn"]});
    let classification = classify_property_value(&getatt_value);
    let preview = classification.get_display_preview(&getatt_value);
    assert_eq!(preview, "!GetAtt MyBucket.Arn");

    // Complex function preview
    let complex_sub = json!({"Fn::Sub": ["Hello ${AWS::Region}", {"param": "value"}]});
    let classification = classify_property_value(&complex_sub);
    let preview = classification.get_display_preview(&complex_sub);
    assert_eq!(preview, "!Sub {...}");

    // Literal string preview (no quotes for cleaner UI)
    let literal = json!("simple string");
    let classification = classify_property_value(&literal);
    let preview = classification.get_display_preview(&literal);
    assert_eq!(preview, "simple string");

    // Long string truncation
    let long_string =
        json!("this is a very long string that should be truncated when displayed in the preview");
    let classification = classify_property_value(&long_string);
    let preview = classification.get_display_preview(&long_string);
    assert!(preview.len() <= 45); // Should be truncated with "..."
    assert!(preview.contains("..."));
}

#[test]
fn test_real_world_cloudformation_examples() {
    // Test with real-world CloudFormation property values

    // S3 bucket notification configuration with references
    let notification_config = json!({
        "LambdaConfigurations": [
            {
                "Event": "s3:ObjectCreated:*",
                "Function": {"Fn::GetAtt": ["ProcessorFunction", "Arn"]}
            }
        ]
    });
    let classification = classify_property_value(&notification_config);
    // Should be classified as complex due to nested GetAtt
    assert_eq!(
        classification.preferred_editor(),
        PreferredEditor::JsonEditor
    );

    // EC2 UserData with Fn::Sub
    let user_data = json!({"Fn::Base64": {"Fn::Sub": [
        "#!/bin/bash\nyum update -y\necho 'Region: ${AWS::Region}' > /tmp/info\necho 'Instance: ${InstanceId}' >> /tmp/info\n",
        {"InstanceId": {"Ref": "AWS::EC2::InstanceId"}}
    ]}});
    let classification = classify_property_value(&user_data);
    assert_eq!(
        classification.preferred_editor(),
        PreferredEditor::JsonEditor
    );

    // Simple resource reference
    let vpc_ref = json!({"Ref": "MyVPC"});
    let classification = classify_property_value(&vpc_ref);
    assert_eq!(
        classification.preferred_editor(),
        PreferredEditor::ReferencePicker
    );

    // ImportValue for cross-stack reference (simple)
    let simple_import_ref = json!({"Fn::ImportValue": "NetworkStack-VPC-ID"});
    let classification = classify_property_value(&simple_import_ref);
    assert_eq!(
        classification.preferred_editor(),
        PreferredEditor::ReferencePicker
    );

    // ImportValue with Sub for complex cross-stack reference
    let complex_import_ref = json!({"Fn::ImportValue": {"Fn::Sub": "${NetworkStack}-VPC-ID"}});
    let classification = classify_property_value(&complex_import_ref);
    assert_eq!(
        classification.preferred_editor(),
        PreferredEditor::JsonEditor
    );
}

#[test]
fn test_edge_cases() {
    // Test edge cases and boundary conditions

    // Empty object
    let empty_obj = json!({});
    let classification = classify_property_value(&empty_obj);
    assert_eq!(classification, PropertyValueClassification::LiteralValue);

    // Empty array
    let empty_array = json!([]);
    let classification = classify_property_value(&empty_array);
    assert_eq!(classification, PropertyValueClassification::LiteralValue);

    // Null value
    let null_value = json!(null);
    let classification = classify_property_value(&null_value);
    assert_eq!(classification, PropertyValueClassification::LiteralValue);

    // Object that looks like intrinsic function but isn't
    let fake_intrinsic = json!({"NotAFunction": "value"});
    let classification = classify_property_value(&fake_intrinsic);
    assert_eq!(classification, PropertyValueClassification::LiteralValue);

    // Malformed intrinsic function (missing required fields)
    let malformed_getatt = json!({"Fn::GetAtt": "not-an-array"});
    let classification = classify_property_value(&malformed_getatt);
    // Should still be detected as GetAtt, even if malformed
    match classification {
        PropertyValueClassification::SimpleReference(IntrinsicFunctionType::GetAtt) => {
            // Expected behavior
        }
        _ => panic!("Expected SimpleReference(GetAtt) even for malformed GetAtt"),
    }
}

#[test]
fn test_condition_functions() {
    // Test CloudFormation condition functions

    let if_condition = json!({"Fn::If": ["IsProduction", "m5.large", "t3.micro"]});
    let classification = classify_property_value(&if_condition);
    assert_eq!(
        classification,
        PropertyValueClassification::ComplexReference(IntrinsicFunctionType::If)
    );
    assert_eq!(
        classification.preferred_editor(),
        PreferredEditor::JsonEditor
    );

    let and_condition = json!({"Fn::And": [
        {"Fn::Equals": [{"Ref": "Environment"}, "production"]},
        {"Fn::Not": [{"Fn::Equals": [{"Ref": "DebugMode"}, "true"]}]}
    ]});
    let classification = classify_property_value(&and_condition);
    assert_eq!(
        classification,
        PropertyValueClassification::ComplexReference(IntrinsicFunctionType::And)
    );
    assert_eq!(
        classification.preferred_editor(),
        PreferredEditor::JsonEditor
    );

    let equals_condition = json!({"Fn::Equals": [{"Ref": "AWS::Region"}, "us-east-1"]});
    let classification = classify_property_value(&equals_condition);
    assert_eq!(
        classification,
        PropertyValueClassification::ComplexReference(IntrinsicFunctionType::Equals)
    );
    assert_eq!(
        classification.preferred_editor(),
        PreferredEditor::JsonEditor
    );
}

#[test]
fn test_single_reference_arrays_should_use_reference_picker() {
    // FAILING TEST: This test captures the specific bug where arrays containing a single
    // reference object are incorrectly classified as complex and sent to JSON editor
    // instead of being recognized as simple reference arrays that should use Reference Picker

    // Single Ref in array - common pattern for CloudFormation properties like AlarmActions
    let single_ref_array = json!([{"Ref": "ScaleDownPolicy"}]);
    let classification = classify_property_value(&single_ref_array);
    // EXPECTED: Should be SimpleReferenceArray -> ReferencePicker for single reference arrays
    assert_eq!(
        classification,
        PropertyValueClassification::SimpleReferenceArray(IntrinsicFunctionType::Ref),
        "Single reference array should be classified as SimpleReferenceArray"
    );
    assert_eq!(
        classification.preferred_editor(),
        PreferredEditor::ReferencePicker,
        "Single reference array should use Reference Picker, not JSON Editor"
    );

    // Single GetAtt in array - another common pattern
    let single_getatt_array = json!([{"Fn::GetAtt": ["MyResource", "Arn"]}]);
    let classification = classify_property_value(&single_getatt_array);
    assert_eq!(
        classification,
        PropertyValueClassification::SimpleReferenceArray(IntrinsicFunctionType::GetAtt),
        "Single GetAtt array should be classified as SimpleReferenceArray"
    );
    assert_eq!(
        classification.preferred_editor(),
        PreferredEditor::ReferencePicker,
        "Single GetAtt array should use Reference Picker, not JSON Editor"
    );

    // Single ImportValue in array
    let single_import_array = json!([{"Fn::ImportValue": "SharedVPC"}]);
    let classification = classify_property_value(&single_import_array);
    assert_eq!(
        classification,
        PropertyValueClassification::SimpleReferenceArray(IntrinsicFunctionType::ImportValue),
        "Single ImportValue array should be classified as SimpleReferenceArray"
    );
    assert_eq!(
        classification.preferred_editor(),
        PreferredEditor::ReferencePicker,
        "Single ImportValue array should use Reference Picker, not JSON Editor"
    );

    // Multiple references in array should still use JSON Editor (complex)
    let multi_ref_array = json!([
        {"Ref": "Policy1"},
        {"Ref": "Policy2"}
    ]);
    let classification = classify_property_value(&multi_ref_array);
    assert_eq!(
        classification.preferred_editor(),
        PreferredEditor::JsonEditor,
        "Multiple reference array should use JSON Editor for complex editing"
    );

    // Mixed content array should use JSON Editor (complex)
    let mixed_array = json!([
        {"Ref": "Policy1"},
        "literal-value"
    ]);
    let classification = classify_property_value(&mixed_array);
    assert_eq!(
        classification.preferred_editor(),
        PreferredEditor::JsonEditor,
        "Mixed content array should use JSON Editor"
    );

    // Array with complex function should use JSON Editor
    let complex_func_array = json!([{"Fn::Sub": ["Hello ${param}", {"param": "value"}]}]);
    let classification = classify_property_value(&complex_func_array);
    assert_eq!(
        classification.preferred_editor(),
        PreferredEditor::JsonEditor,
        "Array with complex function should use JSON Editor"
    );
}

#[test]
fn test_single_reference_array_display_preview() {
    // Test that single reference arrays get appropriate display previews

    let single_ref_array = json!([{"Ref": "MyPolicy"}]);
    let classification = classify_property_value(&single_ref_array);
    let preview = classification.get_display_preview(&single_ref_array);

    // Should show a clear indication that it's a single reference
    // Expected format: something like "!Ref MyPolicy" or "[!Ref MyPolicy]"
    assert!(
        preview.contains("!Ref") && preview.contains("MyPolicy"),
        "Preview should show the reference content clearly: {}",
        preview
    );
}

#[test]
fn test_real_world_cloudformation_single_ref_arrays() {
    // Test real CloudFormation patterns that use single reference arrays

    // CloudWatch Alarm AlarmActions - very common pattern
    let alarm_actions = json!([{"Ref": "ScaleDownPolicy"}]);
    let classification = classify_property_value(&alarm_actions);
    assert_eq!(
        classification.preferred_editor(),
        PreferredEditor::ReferencePicker,
        "CloudWatch AlarmActions with single policy reference should use Reference Picker"
    );

    // Auto Scaling Group TerminationPolicies with single reference
    let termination_policies = json!([{"Ref": "TerminationPolicy"}]);
    let classification = classify_property_value(&termination_policies);
    assert_eq!(
        classification.preferred_editor(),
        PreferredEditor::ReferencePicker,
        "ASG TerminationPolicies with single reference should use Reference Picker"
    );

    // Lambda Environment Variables pointing to parameters
    let env_var_value = json!([{"Ref": "DatabaseUrl"}]);
    let classification = classify_property_value(&env_var_value);
    assert_eq!(
        classification.preferred_editor(),
        PreferredEditor::ReferencePicker,
        "Lambda environment variable array with single reference should use Reference Picker"
    );
}

#[test]
fn test_clean_string_display_without_quotes() {
    // Test the specific examples mentioned - quotes should be removed from button display

    // Test "CPUUtilization" - common CloudWatch metric name
    let cpu_metric = json!("CPUUtilization");
    let classification = classify_property_value(&cpu_metric);
    let preview = classification.get_display_preview(&cpu_metric);
    assert_eq!(
        preview, "CPUUtilization",
        "CPUUtilization should display without quotes"
    );

    // Test "AWS/EC2" - common CloudWatch namespace
    let ec2_namespace = json!("AWS/EC2");
    let classification = classify_property_value(&ec2_namespace);
    let preview = classification.get_display_preview(&ec2_namespace);
    assert_eq!(preview, "AWS/EC2", "AWS/EC2 should display without quotes");

    // Test other common CloudFormation string values
    let bucket_name = json!("my-s3-bucket");
    let classification = classify_property_value(&bucket_name);
    let preview = classification.get_display_preview(&bucket_name);
    assert_eq!(
        preview, "my-s3-bucket",
        "Bucket names should display without quotes"
    );

    // Test empty string
    let empty_string = json!("");
    let classification = classify_property_value(&empty_string);
    let preview = classification.get_display_preview(&empty_string);
    assert_eq!(preview, "", "Empty strings should display as empty");

    // Test string with spaces
    let descriptive_text = json!("Launch Configuration for Web Servers");
    let classification = classify_property_value(&descriptive_text);
    let preview = classification.get_display_preview(&descriptive_text);
    assert_eq!(
        preview, "Launch Configuration for Web Servers",
        "Descriptive text should display without quotes"
    );

    // Test that UI button logic handles empty strings correctly
    assert_eq!(
        classification.preferred_editor(),
        PreferredEditor::ValueEditor,
        "String values should use Value Editor"
    );
}
