//! CloudFormation intrinsic function detection and classification utilities
//!
//! This module provides comprehensive detection of CloudFormation intrinsic functions
//! in both JSON object format and string format, helping to properly classify
//! property values for the correct editing interface (Value Editor vs Reference Picker).

use serde_json::Value;

/// Represents the type of CloudFormation intrinsic function detected
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntrinsicFunctionType {
    /// Simple reference: {"Ref": "ResourceName"} or !Ref ResourceName
    Ref,
    /// Get attribute: {"Fn::GetAtt": ["Resource", "Attribute"]} or !GetAtt Resource.Attribute
    GetAtt,
    /// String substitution: {"Fn::Sub": "text ${var}"} or !Sub "text ${var}"
    Sub,
    /// Join array elements: {"Fn::Join": [",", ["a", "b"]]} or !Join [",", ["a", "b"]]
    Join,
    /// Select from array: {"Fn::Select": [0, array]} or !Select [0, array]
    Select,
    /// Split string: {"Fn::Split": [",", "a,b,c"]} or !Split [",", "a,b,c"]
    Split,
    /// Base64 encode: {"Fn::Base64": "text"} or !Base64 "text"
    Base64,
    /// Find in map: {"Fn::FindInMap": ["Map", "Key1", "Key2"]} or !FindInMap ["Map", "Key1", "Key2"]
    FindInMap,
    /// Import value: {"Fn::ImportValue": "ExportName"} or !ImportValue ExportName
    ImportValue,
    /// Get availability zones: {"Fn::GetAZs": "region"} or !GetAZs region
    GetAZs,
    /// Calculate CIDR: {"Fn::Cidr": ["10.0.0.0/16", 256, 8]} or !Cidr ["10.0.0.0/16", 256, 8]
    Cidr,
    /// Get length: {"Fn::Length": array} or !Length array
    Length,
    /// Convert to JSON string: {"Fn::ToJsonString": object} or !ToJsonString object
    ToJsonString,
    /// Transform: {"Fn::Transform": {...}} or !Transform {...}
    Transform,
    /// Condition functions
    /// If condition: {"Fn::If": ["ConditionName", "TrueValue", "FalseValue"]} or !If ["ConditionName", "TrueValue", "FalseValue"]
    If,
    /// And condition: {"Fn::And": [condition1, condition2]} or !And [condition1, condition2]
    And,
    /// Or condition: {"Fn::Or": [condition1, condition2]} or !Or [condition1, condition2]
    Or,
    /// Not condition: {"Fn::Not": \[condition\]} or !Not \[condition\]
    Not,
    /// Equals condition: {"Fn::Equals": [value1, value2]} or !Equals [value1, value2]
    Equals,
    /// ForEach loop: {"Fn::ForEach": [...]} or !ForEach [...]
    ForEach,
}

impl IntrinsicFunctionType {
    /// Get the JSON function name for this intrinsic function
    pub fn json_name(&self) -> &'static str {
        match self {
            IntrinsicFunctionType::Ref => "Ref",
            IntrinsicFunctionType::GetAtt => "Fn::GetAtt",
            IntrinsicFunctionType::Sub => "Fn::Sub",
            IntrinsicFunctionType::Join => "Fn::Join",
            IntrinsicFunctionType::Select => "Fn::Select",
            IntrinsicFunctionType::Split => "Fn::Split",
            IntrinsicFunctionType::Base64 => "Fn::Base64",
            IntrinsicFunctionType::FindInMap => "Fn::FindInMap",
            IntrinsicFunctionType::ImportValue => "Fn::ImportValue",
            IntrinsicFunctionType::GetAZs => "Fn::GetAZs",
            IntrinsicFunctionType::Cidr => "Fn::Cidr",
            IntrinsicFunctionType::Length => "Fn::Length",
            IntrinsicFunctionType::ToJsonString => "Fn::ToJsonString",
            IntrinsicFunctionType::Transform => "Fn::Transform",
            IntrinsicFunctionType::If => "Fn::If",
            IntrinsicFunctionType::And => "Fn::And",
            IntrinsicFunctionType::Or => "Fn::Or",
            IntrinsicFunctionType::Not => "Fn::Not",
            IntrinsicFunctionType::Equals => "Fn::Equals",
            IntrinsicFunctionType::ForEach => "Fn::ForEach",
        }
    }

    /// Get the YAML short form name for this intrinsic function
    pub fn yaml_short_name(&self) -> &'static str {
        match self {
            IntrinsicFunctionType::Ref => "!Ref",
            IntrinsicFunctionType::GetAtt => "!GetAtt",
            IntrinsicFunctionType::Sub => "!Sub",
            IntrinsicFunctionType::Join => "!Join",
            IntrinsicFunctionType::Select => "!Select",
            IntrinsicFunctionType::Split => "!Split",
            IntrinsicFunctionType::Base64 => "!Base64",
            IntrinsicFunctionType::FindInMap => "!FindInMap",
            IntrinsicFunctionType::ImportValue => "!ImportValue",
            IntrinsicFunctionType::GetAZs => "!GetAZs",
            IntrinsicFunctionType::Cidr => "!Cidr",
            IntrinsicFunctionType::Length => "!Length",
            IntrinsicFunctionType::ToJsonString => "!ToJsonString",
            IntrinsicFunctionType::Transform => "!Transform",
            IntrinsicFunctionType::If => "!If",
            IntrinsicFunctionType::And => "!And",
            IntrinsicFunctionType::Or => "!Or",
            IntrinsicFunctionType::Not => "!Not",
            IntrinsicFunctionType::Equals => "!Equals",
            IntrinsicFunctionType::ForEach => "!ForEach",
        }
    }

    /// Check if this intrinsic function is typically used for references
    /// (vs complex value manipulation that might be better edited as JSON)
    pub fn is_reference_function(&self) -> bool {
        matches!(
            self,
            IntrinsicFunctionType::Ref
                | IntrinsicFunctionType::GetAtt
                | IntrinsicFunctionType::ImportValue
                | IntrinsicFunctionType::GetAZs
        )
    }

    /// Check if this intrinsic function involves complex value manipulation
    /// that might benefit from JSON editing
    pub fn is_complex_function(&self) -> bool {
        matches!(
            self,
            IntrinsicFunctionType::Sub
                | IntrinsicFunctionType::Join
                | IntrinsicFunctionType::Select
                | IntrinsicFunctionType::Split
                | IntrinsicFunctionType::FindInMap
                | IntrinsicFunctionType::Cidr
                | IntrinsicFunctionType::Transform
                | IntrinsicFunctionType::ForEach
                | IntrinsicFunctionType::If
                | IntrinsicFunctionType::And
                | IntrinsicFunctionType::Or
                | IntrinsicFunctionType::Not
                | IntrinsicFunctionType::Equals
        )
    }
}

/// Classification result for a property value
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyValueClassification {
    /// Simple literal value (string, number, boolean, array, plain object)
    LiteralValue,
    /// Simple reference function that should use Reference Picker
    SimpleReference(IntrinsicFunctionType),
    /// Complex intrinsic function that might benefit from JSON editing
    ComplexReference(IntrinsicFunctionType),
    /// String that contains intrinsic function syntax (like "${!Ref Resource}")
    StringWithReferences,
    /// Array containing a single simple reference that should use Reference Picker
    SimpleReferenceArray(IntrinsicFunctionType),
}

impl PropertyValueClassification {
    /// Determine the best editing interface for this classification
    pub fn preferred_editor(&self) -> PreferredEditor {
        match self {
            PropertyValueClassification::LiteralValue => PreferredEditor::ValueEditor,
            PropertyValueClassification::SimpleReference(_) => PreferredEditor::ReferencePicker,
            PropertyValueClassification::ComplexReference(_) => PreferredEditor::JsonEditor,
            PropertyValueClassification::StringWithReferences => PreferredEditor::ValueEditor,
            PropertyValueClassification::SimpleReferenceArray(_) => {
                PreferredEditor::ReferencePicker
            }
        }
    }

    /// Get a display preview for this classification
    pub fn get_display_preview(&self, value: &Value) -> String {
        match self {
            PropertyValueClassification::LiteralValue => format_literal_value_preview(value),
            PropertyValueClassification::SimpleReference(func_type) => {
                format_reference_preview(func_type, value)
            }
            PropertyValueClassification::ComplexReference(func_type) => {
                format!("{} {{...}}", func_type.yaml_short_name())
            }
            PropertyValueClassification::StringWithReferences => {
                if let Value::String(s) = value {
                    if s.len() > 30 {
                        format!("{}...", &s[..30])
                    } else {
                        s.clone()
                    }
                } else {
                    "String with refs".to_string()
                }
            }
            PropertyValueClassification::SimpleReferenceArray(func_type) => {
                // Format single reference arrays with the inner reference preview
                if let Value::Array(arr) = value {
                    if let Some(first_item) = arr.first() {
                        format_reference_preview(func_type, first_item)
                    } else {
                        format!("[{}]", func_type.yaml_short_name())
                    }
                } else {
                    format!("[{}]", func_type.yaml_short_name())
                }
            }
        }
    }
}

/// Preferred editing interface for a property value
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreferredEditor {
    /// Use the ValueEditorWindow for literal values and simple editing
    ValueEditor,
    /// Use the ReferencePickerWindow for simple references
    ReferencePicker,
    /// Use JSON editor mode in ValueEditorWindow for complex structures
    JsonEditor,
}

/// Detect if a JSON value contains a CloudFormation intrinsic function
pub fn detect_intrinsic_function(value: &Value) -> Option<IntrinsicFunctionType> {
    match value {
        Value::Object(obj) => {
            // Check for each possible intrinsic function
            if obj.contains_key("Ref") {
                Some(IntrinsicFunctionType::Ref)
            } else if obj.contains_key("Fn::GetAtt") {
                Some(IntrinsicFunctionType::GetAtt)
            } else if obj.contains_key("Fn::Sub") {
                Some(IntrinsicFunctionType::Sub)
            } else if obj.contains_key("Fn::Join") {
                Some(IntrinsicFunctionType::Join)
            } else if obj.contains_key("Fn::Select") {
                Some(IntrinsicFunctionType::Select)
            } else if obj.contains_key("Fn::Split") {
                Some(IntrinsicFunctionType::Split)
            } else if obj.contains_key("Fn::Base64") {
                Some(IntrinsicFunctionType::Base64)
            } else if obj.contains_key("Fn::FindInMap") {
                Some(IntrinsicFunctionType::FindInMap)
            } else if obj.contains_key("Fn::ImportValue") {
                Some(IntrinsicFunctionType::ImportValue)
            } else if obj.contains_key("Fn::GetAZs") {
                Some(IntrinsicFunctionType::GetAZs)
            } else if obj.contains_key("Fn::Cidr") {
                Some(IntrinsicFunctionType::Cidr)
            } else if obj.contains_key("Fn::Length") {
                Some(IntrinsicFunctionType::Length)
            } else if obj.contains_key("Fn::ToJsonString") {
                Some(IntrinsicFunctionType::ToJsonString)
            } else if obj.contains_key("Fn::Transform") {
                Some(IntrinsicFunctionType::Transform)
            } else if obj.contains_key("Fn::If") {
                Some(IntrinsicFunctionType::If)
            } else if obj.contains_key("Fn::And") {
                Some(IntrinsicFunctionType::And)
            } else if obj.contains_key("Fn::Or") {
                Some(IntrinsicFunctionType::Or)
            } else if obj.contains_key("Fn::Not") {
                Some(IntrinsicFunctionType::Not)
            } else if obj.contains_key("Fn::Equals") {
                Some(IntrinsicFunctionType::Equals)
            } else if obj.contains_key("Fn::ForEach") {
                Some(IntrinsicFunctionType::ForEach)
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Detect if a string contains CloudFormation intrinsic function syntax
pub fn detect_intrinsic_function_in_string(s: &str) -> bool {
    // Look for ${...} patterns that might contain references
    s.contains("${") ||
    // Look for YAML-style function calls (though less common in JSON values)
    s.contains("!Ref ") ||
    s.contains("!GetAtt ") ||
    s.contains("!Sub ") ||
    s.contains("!Join ") ||
    s.contains("!Select ") ||
    s.contains("!Split ") ||
    s.contains("!Base64 ") ||
    s.contains("!FindInMap ") ||
    s.contains("!ImportValue ") ||
    s.contains("!GetAZs") ||
    s.contains("!If ") ||
    s.contains("!And ") ||
    s.contains("!Or ") ||
    s.contains("!Not ") ||
    s.contains("!Equals ")
}

/// Check if an intrinsic function value contains nested intrinsic functions
fn has_nested_intrinsic_functions(value: &Value) -> bool {
    match value {
        Value::Object(obj) => {
            for (_, v) in obj {
                if detect_intrinsic_function(v).is_some() || has_nested_intrinsic_functions(v) {
                    return true;
                }
            }
        }
        Value::Array(arr) => {
            for item in arr {
                if detect_intrinsic_function(item).is_some() || has_nested_intrinsic_functions(item)
                {
                    return true;
                }
            }
        }
        Value::String(s) => {
            if detect_intrinsic_function_in_string(s) {
                return true;
            }
        }
        _ => {}
    }
    false
}

/// Classify a property value to determine the best editing interface
pub fn classify_property_value(value: &Value) -> PropertyValueClassification {
    // First check if it's an intrinsic function object
    if let Some(func_type) = detect_intrinsic_function(value) {
        // Even if it's normally a "simple" reference function, check if it contains nested functions
        if func_type.is_reference_function() {
            // Check if the function value contains nested intrinsic functions
            if let Value::Object(obj) = value {
                for (_, func_value) in obj {
                    if has_nested_intrinsic_functions(func_value) {
                        return PropertyValueClassification::ComplexReference(func_type);
                    }
                }
            }
            return PropertyValueClassification::SimpleReference(func_type);
        } else {
            return PropertyValueClassification::ComplexReference(func_type);
        }
    }

    // Check if it's a string with reference syntax
    if let Value::String(s) = value {
        if detect_intrinsic_function_in_string(s) {
            return PropertyValueClassification::StringWithReferences;
        }
    }

    // Check for arrays that might contain references
    if let Value::Array(arr) = value {
        // Special case: single reference in array - common CloudFormation pattern
        if arr.len() == 1 {
            if let Some(single_item) = arr.first() {
                if let Some(func_type) = detect_intrinsic_function(single_item) {
                    // Check if it's a simple reference function and doesn't have nested complexity
                    if func_type.is_reference_function()
                        && !has_nested_intrinsic_functions(single_item)
                    {
                        return PropertyValueClassification::SimpleReferenceArray(func_type);
                    } else {
                        // Complex function in single-item array still needs JSON editor
                        return PropertyValueClassification::ComplexReference(func_type);
                    }
                }
            }
        }

        // Multiple items or complex content - check each item
        for item in arr {
            if detect_intrinsic_function(item).is_some() || has_nested_intrinsic_functions(item) {
                return PropertyValueClassification::ComplexReference(
                    IntrinsicFunctionType::Select,
                ); // Arbitrary choice for arrays with refs
            }
        }
    }

    // Check for objects that might contain nested references
    if let Value::Object(obj) = value {
        // If it's not an intrinsic function but contains references, it's complex
        for (_, v) in obj {
            if detect_intrinsic_function(v).is_some() || has_nested_intrinsic_functions(v) {
                return PropertyValueClassification::ComplexReference(IntrinsicFunctionType::Sub);
                // Arbitrary choice for objects with refs
            }
        }
    }

    // Default to literal value
    PropertyValueClassification::LiteralValue
}

/// Format a literal value for preview display
fn format_literal_value_preview(value: &Value) -> String {
    match value {
        Value::String(s) => {
            if s.is_empty() {
                String::new()
            } else if s.len() > 40 {
                format!("{}...", &s[..40])
            } else {
                s.clone()
            }
        }
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Array(arr) => format!("[Array: {} items]", arr.len()),
        Value::Object(obj) => format!("{{Object: {} keys}}", obj.len()),
        Value::Null => "null".to_string(),
    }
}

/// Format a reference function for preview display
fn format_reference_preview(func_type: &IntrinsicFunctionType, value: &Value) -> String {
    match func_type {
        IntrinsicFunctionType::Ref => {
            if let Value::Object(obj) = value {
                if let Some(Value::String(resource)) = obj.get("Ref") {
                    format!("!Ref {}", resource)
                } else {
                    "!Ref".to_string()
                }
            } else {
                "!Ref".to_string()
            }
        }
        IntrinsicFunctionType::GetAtt => {
            if let Value::Object(obj) = value {
                if let Some(Value::Array(arr)) = obj.get("Fn::GetAtt") {
                    if arr.len() >= 2 {
                        if let (Some(Value::String(resource)), Some(Value::String(attr))) =
                            (arr.first(), arr.get(1))
                        {
                            format!("!GetAtt {}.{}", resource, attr)
                        } else {
                            "!GetAtt".to_string()
                        }
                    } else {
                        "!GetAtt".to_string()
                    }
                } else {
                    "!GetAtt".to_string()
                }
            } else {
                "!GetAtt".to_string()
            }
        }
        IntrinsicFunctionType::ImportValue => {
            if let Value::Object(obj) = value {
                if let Some(Value::String(export_name)) = obj.get("Fn::ImportValue") {
                    format!("!ImportValue {}", export_name)
                } else {
                    "!ImportValue".to_string()
                }
            } else {
                "!ImportValue".to_string()
            }
        }
        IntrinsicFunctionType::GetAZs => "!GetAZs".to_string(),
        _ => func_type.yaml_short_name().to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_detect_ref_function() {
        let value = json!({"Ref": "MyResource"});
        assert_eq!(
            detect_intrinsic_function(&value),
            Some(IntrinsicFunctionType::Ref)
        );
    }

    #[test]
    fn test_detect_getatt_function() {
        let value = json!({"Fn::GetAtt": ["MyResource", "Arn"]});
        assert_eq!(
            detect_intrinsic_function(&value),
            Some(IntrinsicFunctionType::GetAtt)
        );
    }

    #[test]
    fn test_detect_sub_function() {
        let value = json!({"Fn::Sub": "Hello ${AWS::Region}"});
        assert_eq!(
            detect_intrinsic_function(&value),
            Some(IntrinsicFunctionType::Sub)
        );
    }

    #[test]
    fn test_classify_literal_string() {
        let value = json!("simple string");
        assert_eq!(
            classify_property_value(&value),
            PropertyValueClassification::LiteralValue
        );
    }

    #[test]
    fn test_classify_simple_reference() {
        let value = json!({"Ref": "MyResource"});
        assert_eq!(
            classify_property_value(&value),
            PropertyValueClassification::SimpleReference(IntrinsicFunctionType::Ref)
        );
    }

    #[test]
    fn test_classify_complex_reference() {
        let value = json!({"Fn::Sub": ["Hello ${param}", {"param": {"Ref": "MyParam"}}]});
        assert_eq!(
            classify_property_value(&value),
            PropertyValueClassification::ComplexReference(IntrinsicFunctionType::Sub)
        );
    }

    #[test]
    fn test_classify_string_with_references() {
        let value = json!("Hello ${AWS::Region}");
        assert_eq!(
            classify_property_value(&value),
            PropertyValueClassification::StringWithReferences
        );
    }

    #[test]
    fn test_detect_intrinsic_in_string() {
        assert!(detect_intrinsic_function_in_string("${AWS::Region}"));
        assert!(detect_intrinsic_function_in_string("!Ref MyResource"));
        assert!(!detect_intrinsic_function_in_string("simple string"));
    }

    #[test]
    fn test_format_ref_preview() {
        let value = json!({"Ref": "MyResource"});
        let classification = classify_property_value(&value);
        assert_eq!(
            classification.get_display_preview(&value),
            "!Ref MyResource"
        );
    }

    #[test]
    fn test_format_getatt_preview() {
        let value = json!({"Fn::GetAtt": ["MyResource", "Arn"]});
        let classification = classify_property_value(&value);
        assert_eq!(
            classification.get_display_preview(&value),
            "!GetAtt MyResource.Arn"
        );
    }

    #[test]
    fn test_preferred_editor_classification() {
        let literal = json!("simple string");
        assert_eq!(
            classify_property_value(&literal).preferred_editor(),
            PreferredEditor::ValueEditor
        );

        let simple_ref = json!({"Ref": "MyResource"});
        assert_eq!(
            classify_property_value(&simple_ref).preferred_editor(),
            PreferredEditor::ReferencePicker
        );

        let complex_ref = json!({"Fn::Sub": ["Hello ${param}", {"param": "value"}]});
        assert_eq!(
            classify_property_value(&complex_ref).preferred_editor(),
            PreferredEditor::JsonEditor
        );
    }
}
