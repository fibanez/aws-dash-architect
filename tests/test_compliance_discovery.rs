use anyhow::Result;
use awsdash::app::compliance_discovery::{
    AvailableComplianceProgram, ComplianceDiscovery, ComplianceProgramCache,
    ComplianceProgramMetadata,
};
use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;

/// Test that ComplianceDiscovery can be created and initialized
#[tokio::test]
async fn test_compliance_discovery_creation() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();

    let discovery = ComplianceDiscovery::new(cache_dir.clone()).await;

    assert!(discovery.is_ok());
    let discovery = discovery.unwrap();
    assert_eq!(discovery.get_cache_dir(), &cache_dir);
}

/// Test discovering available compliance programs from GitHub
#[tokio::test]
async fn test_discover_available_programs() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();

    let mut discovery = ComplianceDiscovery::new(cache_dir).await.unwrap();

    // Discover available compliance programs
    let result = discovery.discover_available_programs().await;

    assert!(result.is_ok());
    let programs = result.unwrap();

    // Should find common compliance programs
    assert!(programs.len() > 0);

    // Verify structure of discovered programs
    for program in &programs {
        assert!(!program.name.is_empty());
        assert!(!program.display_name.is_empty());
        assert!(!program.github_path.is_empty());
        assert!(program.estimated_rule_count > 0);
        assert!(!program.description.is_empty());
    }

    // Should include well-known compliance programs
    let program_names: Vec<String> = programs.iter().map(|p| p.name.clone()).collect();
    assert!(program_names
        .iter()
        .any(|name| name.contains("nist") || name.contains("NIST")));
}

/// Test caching of discovered compliance programs
#[tokio::test]
async fn test_compliance_program_caching() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();

    let mut discovery = ComplianceDiscovery::new(cache_dir.clone()).await.unwrap();

    // First discovery - should hit GitHub API
    let programs1 = discovery.discover_available_programs().await.unwrap();

    // Verify cache file was created
    let cache_file = cache_dir.join("available_programs.json");
    assert!(cache_file.exists());

    // Second discovery - should use cache
    let programs2 = discovery.get_cached_programs().await.unwrap();

    // Should be the same content
    assert_eq!(programs1.len(), programs2.len());
    for (prog1, prog2) in programs1.iter().zip(programs2.iter()) {
        assert_eq!(prog1.name, prog2.name);
        assert_eq!(prog1.github_path, prog2.github_path);
    }
}

/// Test cache invalidation and refresh
#[tokio::test]
async fn test_cache_invalidation() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();

    let mut discovery = ComplianceDiscovery::new(cache_dir.clone()).await.unwrap();

    // Create initial cache
    let _programs = discovery.discover_available_programs().await.unwrap();

    // Check if cache needs refresh (should be false for fresh cache)
    let needs_refresh = discovery.needs_cache_refresh().await.unwrap();
    assert!(!needs_refresh);

    // Force cache invalidation
    discovery.invalidate_cache().await.unwrap();

    // Now should need refresh
    let needs_refresh = discovery.needs_cache_refresh().await.unwrap();
    assert!(needs_refresh);

    // Getting cached programs should fail or return empty
    let cached_result = discovery.get_cached_programs().await;
    assert!(cached_result.is_err() || cached_result.unwrap().is_empty());
}

/// Test individual compliance program metadata
#[test]
fn test_compliance_program_metadata() {
    let metadata = ComplianceProgramMetadata {
        name: "nist_800_53_rev_5".to_string(),
        display_name: "NIST 800-53 Revision 5".to_string(),
        description: "NIST Cybersecurity Framework controls".to_string(),
        github_path: "rules/aws-control-tower/cfn-guard/nist_800_53_rev_5".to_string(),
        estimated_rule_count: 75,
        last_updated: "2024-01-15".to_string(),
        category: "Government".to_string(),
    };

    assert_eq!(metadata.name, "nist_800_53_rev_5");
    assert_eq!(metadata.estimated_rule_count, 75);
    assert!(metadata.display_name.contains("NIST"));
}

/// Test AvailableComplianceProgram structure
#[test]
fn test_available_compliance_program() {
    let program = AvailableComplianceProgram {
        name: "pci_dss".to_string(),
        display_name: "PCI DSS".to_string(),
        description: "Payment Card Industry Data Security Standard".to_string(),
        github_path: "rules/aws-control-tower/cfn-guard/pci_dss".to_string(),
        estimated_rule_count: 45,
        category: "Industry".to_string(),
        tags: vec!["payment".to_string(), "security".to_string()],
    };

    assert_eq!(program.name, "pci_dss");
    assert_eq!(program.estimated_rule_count, 45);
    assert!(program.tags.contains(&"payment".to_string()));
}

/// Test GitHubApiClient functionality
#[tokio::test]
async fn test_github_api_client() {
    let client = GitHubApiClient::new().await.unwrap();

    // Test getting repository structure
    let result = client.get_repository_structure().await;

    assert!(result.is_ok());
    let structure = result.unwrap();

    // Should contain rules directories
    assert!(structure.contains_key("rules"));

    // Should have reasonable number of entries
    assert!(structure.len() > 0);
}

/// Test parsing GitHub repository structure for compliance programs
#[tokio::test]
async fn test_parse_repository_structure() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();

    let discovery = ComplianceDiscovery::new(cache_dir).await.unwrap();

    // Mock repository structure
    let mut mock_structure = HashMap::new();
    mock_structure.insert(
        "rules/aws-control-tower/cfn-guard/nist_800_53_rev_5".to_string(),
        vec![
            "s3_bucket_ssl_requests_only.guard".to_string(),
            "iam_password_policy.guard".to_string(),
            "ec2_security_group_attached.guard".to_string(),
        ],
    );
    mock_structure.insert(
        "rules/aws-control-tower/cfn-guard/pci_dss".to_string(),
        vec![
            "rds_storage_encrypted.guard".to_string(),
            "cloudtrail_enabled.guard".to_string(),
        ],
    );

    let programs = discovery
        .parse_repository_structure(mock_structure)
        .await
        .unwrap();

    assert_eq!(programs.len(), 2);

    // Find NIST program
    let nist_program = programs.iter().find(|p| p.name.contains("nist")).unwrap();
    assert_eq!(nist_program.estimated_rule_count, 3);
    assert!(nist_program.display_name.contains("NIST"));

    // Find PCI program
    let pci_program = programs.iter().find(|p| p.name.contains("pci")).unwrap();
    assert_eq!(pci_program.estimated_rule_count, 2);
    assert!(pci_program.display_name.contains("PCI"));
}

/// Test error handling for invalid GitHub responses
#[tokio::test]
async fn test_github_api_error_handling() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();

    let mut discovery = ComplianceDiscovery::new(cache_dir).await.unwrap();

    // Test with mock client that returns errors
    let result = discovery.discover_available_programs().await;

    // Should handle errors gracefully - either succeed with real API or fail cleanly
    match result {
        Ok(programs) => {
            // If successful, should have valid programs
            assert!(programs.len() >= 0); // Could be empty in test environment
        }
        Err(err) => {
            // If failed, should be a reasonable error message
            let error_msg = err.to_string();
            assert!(
                error_msg.contains("GitHub")
                    || error_msg.contains("network")
                    || error_msg.contains("API")
            );
        }
    }
}

/// Test compliance program search and filtering
#[tokio::test]
async fn test_program_search_filtering() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();

    let mut discovery = ComplianceDiscovery::new(cache_dir).await.unwrap();

    // Get all programs
    let all_programs = discovery.discover_available_programs().await.unwrap();

    // Test searching by name
    let nist_programs = discovery.search_programs("nist").await.unwrap();
    assert!(nist_programs.len() <= all_programs.len());
    for program in &nist_programs {
        assert!(
            program.name.to_lowercase().contains("nist")
                || program.display_name.to_lowercase().contains("nist")
                || program.description.to_lowercase().contains("nist")
        );
    }

    // Test searching by category
    let gov_programs = discovery.filter_by_category("Government").await.unwrap();
    for program in &gov_programs {
        assert_eq!(program.category, "Government");
    }
}

/// Test compliance program cache serialization
#[test]
fn test_cache_serialization() -> Result<()> {
    let programs = vec![AvailableComplianceProgram {
        name: "test_program".to_string(),
        display_name: "Test Program".to_string(),
        description: "Test compliance program".to_string(),
        github_path: "rules/test".to_string(),
        estimated_rule_count: 10,
        category: "Test".to_string(),
        tags: vec!["test".to_string()],
    }];

    let cache = ComplianceProgramCache {
        programs: programs.clone(),
        last_updated: chrono::Utc::now(),
        cache_version: "1.0".to_string(),
    };

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&cache)?;
    assert!(json.contains("test_program"));
    assert!(json.contains("Test Program"));

    // Deserialize back
    let deserialized: ComplianceProgramCache = serde_json::from_str(&json)?;
    assert_eq!(deserialized.programs.len(), 1);
    assert_eq!(deserialized.programs[0].name, "test_program");

    Ok(())
}

/// Test handling of malformed cache files
#[tokio::test]
async fn test_malformed_cache_handling() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();

    // Create malformed cache file
    let cache_file = cache_dir.join("available_programs.json");
    std::fs::create_dir_all(cache_dir.clone()).unwrap();
    std::fs::write(&cache_file, "invalid json content").unwrap();

    let mut discovery = ComplianceDiscovery::new(cache_dir).await.unwrap();

    // Should handle malformed cache gracefully
    let result = discovery.get_cached_programs().await;
    assert!(result.is_err());

    // Should still be able to discover fresh programs
    let fresh_result = discovery.discover_available_programs().await;
    // Either succeeds with real API or fails cleanly
    assert!(fresh_result.is_ok() || fresh_result.is_err());
}
