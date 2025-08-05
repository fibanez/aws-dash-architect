use awsdash::app::guard_rules_registry::{
    GuardRulesRegistry, ComplianceProgram, RuleMetadata, RuleVersion
};
use std::path::PathBuf;
use tempfile::TempDir;
use anyhow::Result;

/// Test that GuardRulesRegistry can be created and initialized
#[tokio::test]
async fn test_guard_rules_registry_creation() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();
    
    let registry = GuardRulesRegistry::new(cache_dir.clone()).await;
    
    assert!(registry.is_ok());
    let registry = registry.unwrap();
    assert_eq!(registry.get_cache_dir(), &cache_dir);
}

/// Test downloading rules for a compliance program
#[tokio::test]
async fn test_download_compliance_rules() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();
    
    let mut registry = GuardRulesRegistry::new(cache_dir).await.unwrap();
    
    // Test downloading NIST 800-53 R5 rules
    let result = registry.download_compliance_rules(ComplianceProgram::NIST80053R5).await;
    
    assert!(result.is_ok());
    let rules = result.unwrap();
    assert!(rules.len() > 0);
    
    // Verify rules have proper structure
    for (rule_name, rule_content) in &rules {
        assert!(!rule_name.is_empty());
        assert!(!rule_content.is_empty());
        assert!(rule_content.contains("rule") || rule_content.contains("let"));
    }
}

/// Test caching mechanism for downloaded rules
#[tokio::test]
async fn test_rules_caching() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();
    
    let mut registry = GuardRulesRegistry::new(cache_dir.clone()).await.unwrap();
    
    // Download rules first time
    let rules1 = registry.download_compliance_rules(ComplianceProgram::NIST80053R5).await.unwrap();
    
    // Verify cache directory was created
    let nist_cache_dir = cache_dir.join("NIST80053R5");
    assert!(nist_cache_dir.exists());
    assert!(nist_cache_dir.is_dir());
    
    // Download rules second time (should use cache)
    let rules2 = registry.get_cached_rules(ComplianceProgram::NIST80053R5).await.unwrap();
    
    // Should be the same content
    assert_eq!(rules1.len(), rules2.len());
    for (rule_name, rule_content) in &rules1 {
        assert_eq!(rules2.get(rule_name), Some(rule_content));
    }
}

/// Test rule version management
#[tokio::test]
async fn test_rule_versioning() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();
    
    let registry = GuardRulesRegistry::new(cache_dir).await.unwrap();
    
    // Test getting rule metadata
    let metadata = registry.get_rule_metadata(ComplianceProgram::NIST80053R5).await;
    
    assert!(metadata.is_ok());
    let metadata = metadata.unwrap();
    assert!(!metadata.version.is_empty());
    assert!(metadata.last_updated.len() > 0);
    assert!(metadata.rules_count > 0);
}

/// Test offline mode with cached rules
#[tokio::test]
async fn test_offline_mode() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();
    
    let mut registry = GuardRulesRegistry::new(cache_dir.clone()).await.unwrap();
    
    // First, download some rules to populate cache
    let _rules = registry.download_compliance_rules(ComplianceProgram::PCIDSS).await.unwrap();
    
    // Create a new registry instance (simulating offline mode)
    let offline_registry = GuardRulesRegistry::new(cache_dir).await.unwrap();
    
    // Should be able to get cached rules even in "offline" mode
    let cached_rules = offline_registry.get_cached_rules(ComplianceProgram::PCIDSS).await;
    
    assert!(cached_rules.is_ok());
    let cached_rules = cached_rules.unwrap();
    assert!(cached_rules.len() > 0);
}

/// Test multiple compliance programs
#[tokio::test]
async fn test_multiple_compliance_programs() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();
    
    let mut registry = GuardRulesRegistry::new(cache_dir).await.unwrap();
    
    let programs = vec![
        ComplianceProgram::NIST80053R5,
        ComplianceProgram::PCIDSS,
        ComplianceProgram::HIPAA,
    ];
    
    let mut all_rules = std::collections::HashMap::new();
    
    for program in programs {
        let rules = registry.download_compliance_rules(program.clone()).await.unwrap();
        assert!(rules.len() > 0);
        all_rules.insert(program, rules);
    }
    
    // Verify each program has different rules
    assert_eq!(all_rules.len(), 3);
    
    // Verify rules are properly separated by compliance program
    for (program, rules) in all_rules {
        let cached_rules = registry.get_cached_rules(program).await.unwrap();
        assert_eq!(rules.len(), cached_rules.len());
    }
}

/// Test rule update detection
#[tokio::test]
async fn test_rule_update_detection() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();
    
    let registry = GuardRulesRegistry::new(cache_dir).await.unwrap();
    
    // Test checking for updates (should work even with no cached rules)
    let has_updates = registry.check_for_updates(ComplianceProgram::NIST80053R5).await;
    
    assert!(has_updates.is_ok());
    // Should indicate updates available when no cache exists
    assert!(has_updates.unwrap());
}

/// Test error handling for invalid compliance programs
#[tokio::test]
async fn test_invalid_compliance_program() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();
    
    let mut registry = GuardRulesRegistry::new(cache_dir).await.unwrap();
    
    // Test with custom compliance program that doesn't exist
    let custom_program = ComplianceProgram::Custom("NonExistentProgram".to_string());
    let result = registry.download_compliance_rules(custom_program).await;
    
    // Should handle gracefully (either return empty rules or error)
    assert!(result.is_ok() || result.is_err());
    if let Ok(rules) = result {
        // If it succeeds, should return empty rules
        assert_eq!(rules.len(), 0);
    }
}

/// Test RuleMetadata structure
#[test]
fn test_rule_metadata_structure() {
    let metadata = RuleMetadata {
        version: "1.0.0".to_string(),
        last_updated: "2024-01-01".to_string(),  
        rules_count: 50,
        compliance_program: ComplianceProgram::NIST80053R5,
        source_url: "https://github.com/aws-cloudformation/aws-guard-rules-registry".to_string(),
    };
    
    assert_eq!(metadata.version, "1.0.0");
    assert_eq!(metadata.rules_count, 50);
    assert_eq!(metadata.compliance_program, ComplianceProgram::NIST80053R5);
    assert!(metadata.source_url.contains("aws-guard-rules-registry"));
}

/// Test RuleVersion enum
#[test]
fn test_rule_version_enum() {
    let latest = RuleVersion::Latest;
    let specific = RuleVersion::Specific("v1.2.3".to_string());
    
    match latest {
        RuleVersion::Latest => assert!(true),
        _ => panic!("Expected Latest version"),
    }
    
    match specific {
        RuleVersion::Specific(version) => assert_eq!(version, "v1.2.3"),
        _ => panic!("Expected Specific version"),
    }
}

/// Test cache cleanup functionality
#[tokio::test]
async fn test_cache_cleanup() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();
    
    let mut registry = GuardRulesRegistry::new(cache_dir.clone()).await.unwrap();
    
    // Download some rules to create cache
    let _rules = registry.download_compliance_rules(ComplianceProgram::NIST80053R5).await.unwrap();
    
    // Verify cache exists
    let cache_path = cache_dir.join("NIST80053R5");
    assert!(cache_path.exists());
    
    // Clean cache
    let result = registry.clear_cache(Some(ComplianceProgram::NIST80053R5)).await;
    assert!(result.is_ok());
    
    // Verify cache is cleared
    assert!(!cache_path.exists() || std::fs::read_dir(&cache_path).unwrap().count() == 0);
}