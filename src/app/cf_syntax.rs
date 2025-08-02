use egui_code_editor::Syntax;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

/// CloudFormation syntax configuration loaded from embedded JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudFormationSyntaxConfig {
    pub name: String,
    pub file_extensions: Vec<String>,
    pub comment_tokens: CommentTokens,
    pub keywords: Vec<String>,
    pub types: Vec<String>,
    pub functions: Vec<String>,
    pub special: Vec<String>,
    pub aws_resource_types: Vec<String>,
    pub punctuation: Punctuation,
    pub string_delimiters: Vec<String>,
    pub numeric_patterns: Vec<String>,
    pub token_rules: TokenRules,
    pub highlighting_priority: Vec<String>,
    pub case_sensitive: bool,
    pub multiline_strings: MultilineStrings,
    pub additional_patterns: AdditionalPatterns,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommentTokens {
    pub line: Option<String>,
    pub block_start: Option<String>,
    pub block_end: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Punctuation {
    pub pairs: Vec<[String; 2]>,
    pub special: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRules {
    pub resource_type_pattern: String,
    pub intrinsic_function_pattern: String,
    pub pseudo_parameter_pattern: String,
    pub parameter_type_pattern: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultilineStrings {
    pub yaml_literal: MultilineStringType,
    pub yaml_folded: MultilineStringType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultilineStringType {
    pub start: String,
    pub indent_based: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdditionalPatterns {
    pub template_version: String,
    pub common_attributes: Vec<String>,
    pub deletion_policies: Vec<String>,
}

/// Embedded CloudFormation syntax configuration
static CF_SYNTAX_JSON: &str = include_str!("../../docs/cf-syntax.json");

/// Lazily loaded CloudFormation syntax configuration
static CF_SYNTAX_CONFIG: Lazy<CloudFormationSyntaxConfig> = Lazy::new(
    || match serde_json::from_str::<CloudFormationSyntaxConfig>(CF_SYNTAX_JSON) {
        Ok(config) => {
            tracing::debug!(
                "CloudFormation syntax loaded: {} keywords, {} functions, {} AWS resource types",
                config.keywords.len(),
                config.functions.len(),
                config.aws_resource_types.len()
            );
            config
        }
        Err(e) => {
            tracing::error!("Failed to parse CloudFormation syntax configuration: {}", e);
            panic!(
                "Failed to parse embedded CloudFormation syntax configuration: {}",
                e
            );
        }
    },
);

/// Create a CloudFormation syntax highlighter
pub fn cloudformation_syntax() -> Syntax {
    let config = &*CF_SYNTAX_CONFIG;

    // Separate CloudFormation terms into appropriate highlighting categories

    // 1. Template-level keywords (blue/keyword color)
    let mut cf_keywords = std::collections::BTreeSet::new();
    for keyword in &config.keywords {
        cf_keywords.insert(keyword.as_str());
    }

    // 2. AWS resource types and parameter types (green/type color)
    let mut cf_types = std::collections::BTreeSet::new();
    for aws_type in &config.aws_resource_types {
        cf_types.insert(aws_type.as_str());
    }
    for param_type in &config.types {
        cf_types.insert(param_type.as_str());
    }

    // 3. Intrinsic functions and special tokens (purple/special color)
    let mut cf_special = std::collections::BTreeSet::new();
    for function in &config.functions {
        cf_special.insert(function.as_str());
    }
    for special in &config.special {
        cf_special.insert(special.as_str());
    }

    // Create the syntax with CloudFormation-specific highlighting
    Syntax::new("CloudFormation")
        .with_comment("#") // YAML/JSON comment style
        .with_keywords(cf_keywords) // Template keywords: blue
        .with_types(cf_types) // AWS resource types: green
        .with_special(cf_special) // Functions and pseudo params: purple
}

/// Create a CloudFormation JSON syntax highlighter optimized for JSON templates
pub fn cloudformation_json_syntax() -> Syntax {
    let config = &*CF_SYNTAX_CONFIG;

    // Separate CloudFormation terms into appropriate highlighting categories

    // 1. Template-level keywords (blue/keyword color)
    let mut cf_keywords = std::collections::BTreeSet::new();
    for keyword in &config.keywords {
        cf_keywords.insert(keyword.as_str());
    }
    // Add JSON literals
    cf_keywords.insert("true");
    cf_keywords.insert("false");
    cf_keywords.insert("null");

    // 2. AWS resource types and parameter types (green/type color)
    let mut cf_types = std::collections::BTreeSet::new();
    for aws_type in &config.aws_resource_types {
        cf_types.insert(aws_type.as_str());
    }
    for param_type in &config.types {
        cf_types.insert(param_type.as_str());
    }

    // 3. Intrinsic functions and special tokens (purple/special color)
    let mut cf_special = std::collections::BTreeSet::new();
    for function in &config.functions {
        cf_special.insert(function.as_str());
    }
    for special in &config.special {
        cf_special.insert(special.as_str());
    }

    Syntax::new("CloudFormation-JSON")
        .with_keywords(cf_keywords) // Template keywords: blue
        .with_types(cf_types) // AWS resource types: green
        .with_special(cf_special) // Functions and pseudo params: purple
}

/// Create a CloudFormation YAML syntax highlighter optimized for YAML templates
pub fn cloudformation_yaml_syntax() -> Syntax {
    let config = &*CF_SYNTAX_CONFIG;

    // Separate CloudFormation terms into appropriate highlighting categories

    // 1. Template-level keywords (blue/keyword color)
    let mut cf_keywords = std::collections::BTreeSet::new();
    for keyword in &config.keywords {
        cf_keywords.insert(keyword.as_str());
    }
    // Add YAML literals
    cf_keywords.insert("true");
    cf_keywords.insert("false");
    cf_keywords.insert("null");
    cf_keywords.insert("yes");
    cf_keywords.insert("no");
    cf_keywords.insert("on");
    cf_keywords.insert("off");

    // 2. AWS resource types and parameter types (green/type color)
    let mut cf_types = std::collections::BTreeSet::new();
    for aws_type in &config.aws_resource_types {
        cf_types.insert(aws_type.as_str());
    }
    for param_type in &config.types {
        cf_types.insert(param_type.as_str());
    }

    // 3. Intrinsic functions and special tokens (purple/special color)
    let mut cf_special = std::collections::BTreeSet::new();
    for function in &config.functions {
        cf_special.insert(function.as_str());
    }
    for special in &config.special {
        cf_special.insert(special.as_str());
    }

    Syntax::new("CloudFormation-YAML")
        .with_comment("#") // YAML comment style
        .with_keywords(cf_keywords) // Template keywords: blue
        .with_types(cf_types) // AWS resource types: green
        .with_special(cf_special) // Functions and pseudo params: purple
}

/// Get the CloudFormation syntax configuration for external use
pub fn get_cf_syntax_config() -> &'static CloudFormationSyntaxConfig {
    &CF_SYNTAX_CONFIG
}

/// Create a test syntax using built-in Rust syntax for comparison
pub fn rust_syntax_for_comparison() -> Syntax {
    egui_code_editor::Syntax::rust()
}

/// Create a simple syntax for testing
pub fn simple_test_syntax() -> Syntax {
    egui_code_editor::Syntax::simple("//")
}

/// Determine appropriate CloudFormation syntax based on content or filename
pub fn detect_cloudformation_syntax(filename: Option<&str>, content: Option<&str>) -> Syntax {
    // Try to detect based on filename extension
    if let Some(fname) = filename {
        let fname_lower = fname.to_lowercase();
        if fname_lower.ends_with(".json") || fname_lower.ends_with(".template") {
            return cloudformation_json_syntax();
        } else if fname_lower.ends_with(".yaml") || fname_lower.ends_with(".yml") {
            return cloudformation_yaml_syntax();
        }
    }

    // Try to detect based on content
    if let Some(content_str) = content {
        let trimmed = content_str.trim_start();
        if trimmed.starts_with('{') {
            return cloudformation_json_syntax();
        } else if trimmed.contains("AWSTemplateFormatVersion") || trimmed.contains("Resources:") {
            return cloudformation_yaml_syntax();
        }
    }

    // Default to JSON syntax for CloudFormation
    cloudformation_json_syntax()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cf_syntax_config_loads() {
        let config = get_cf_syntax_config();
        assert_eq!(config.name, "cloudformation");
        assert!(!config.keywords.is_empty());
        assert!(!config.aws_resource_types.is_empty());
        assert!(config.case_sensitive);
    }

    #[test]
    fn test_syntax_detection() {
        // Test JSON detection
        let json_syntax = detect_cloudformation_syntax(Some("template.json"), None);
        assert_eq!(json_syntax.language(), "CloudFormation-JSON");

        // Test YAML detection
        let yaml_syntax = detect_cloudformation_syntax(Some("template.yaml"), None);
        assert_eq!(yaml_syntax.language(), "CloudFormation-YAML");

        // Test content-based detection
        let json_content_syntax = detect_cloudformation_syntax(
            None,
            Some("{ \"AWSTemplateFormatVersion\": \"2010-09-09\" }"),
        );
        assert_eq!(json_content_syntax.language(), "CloudFormation-JSON");
    }

    #[test]
    fn test_syntax_creation() {
        let cf_syntax = cloudformation_syntax();
        assert_eq!(cf_syntax.language(), "CloudFormation");

        let json_syntax = cloudformation_json_syntax();
        assert_eq!(json_syntax.language(), "CloudFormation-JSON");

        let yaml_syntax = cloudformation_yaml_syntax();
        assert_eq!(yaml_syntax.language(), "CloudFormation-YAML");
    }
}
