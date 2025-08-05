use awsdash::app::bulk_rule_downloader::{
    BulkRuleDownloader, RuleDownloadManager, DownloadProgress, DownloadStatus,
    RuleStorage, RuleIndex, ComplianceRuleSet, GuardRuleFile
};
use awsdash::app::compliance_discovery::AvailableComplianceProgram;
use anyhow::Result;
use std::collections::HashMap;
use tempfile::TempDir;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Test that BulkRuleDownloader can be created and initialized
#[tokio::test]
async fn test_bulk_rule_downloader_creation() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();
    
    let downloader = BulkRuleDownloader::new(cache_dir.clone()).await;
    
    assert!(downloader.is_ok());
    let downloader = downloader.unwrap();
    assert_eq!(downloader.get_storage_dir(), &cache_dir);
}

/// Test downloading rules for a single compliance program
#[tokio::test]
async fn test_download_single_compliance_program() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();
    
    let mut downloader = BulkRuleDownloader::new(cache_dir.clone()).await.unwrap();
    
    let program = AvailableComplianceProgram {
        name: "nist_800_53_rev_5".to_string(),
        display_name: "NIST 800-53 Revision 5".to_string(),
        description: "NIST cybersecurity framework".to_string(),
        github_path: "rules/aws-control-tower/cfn-guard/nist_800_53_rev_5".to_string(),
        estimated_rule_count: 75,
        category: "Government".to_string(),
        tags: vec!["government".to_string(), "cybersecurity".to_string()],
    };
    
    // Download rules for the program
    let result = downloader.download_compliance_program_rules(&program).await;
    
    assert!(result.is_ok());
    let rule_set = result.unwrap();
    
    // Verify downloaded rule set
    assert_eq!(rule_set.program_name, "nist_800_53_rev_5");
    assert!(rule_set.rules.len() > 0);
    assert!(!rule_set.version.is_empty());
    assert!(!rule_set.source_url.is_empty());
    
    // Verify rules have proper structure
    for (rule_name, rule_file) in &rule_set.rules {
        assert!(!rule_name.is_empty());
        assert!(!rule_file.content.is_empty());
        assert!(!rule_file.file_path.is_empty());
        assert!(rule_file.content.contains("rule") || rule_file.content.contains("let"));
    }
    
    // Verify files were saved to storage
    let program_dir = cache_dir.join(&program.name);
    assert!(program_dir.exists());
    assert!(program_dir.is_dir());
}

/// Test bulk downloading multiple compliance programs
#[tokio::test]
async fn test_bulk_download_multiple_programs() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();
    
    let mut downloader = BulkRuleDownloader::new(cache_dir.clone()).await.unwrap();
    
    let programs = vec![
        AvailableComplianceProgram {
            name: "nist_800_53_rev_5".to_string(),
            display_name: "NIST 800-53 Revision 5".to_string(),
            description: "NIST cybersecurity framework".to_string(),
            github_path: "rules/aws-control-tower/cfn-guard/nist_800_53_rev_5".to_string(),
            estimated_rule_count: 75,
            category: "Government".to_string(),
            tags: vec!["government".to_string()],
        },
        AvailableComplianceProgram {
            name: "pci_dss".to_string(),
            display_name: "PCI DSS".to_string(),
            description: "Payment Card Industry".to_string(),
            github_path: "rules/aws-control-tower/cfn-guard/pci_dss".to_string(),
            estimated_rule_count: 45,
            category: "Industry".to_string(),
            tags: vec!["payment".to_string()],
        },
    ];
    
    // Download all programs
    let result = downloader.download_multiple_programs(&programs).await;
    
    assert!(result.is_ok());
    let rule_sets = result.unwrap();
    
    // Should have downloaded both programs
    assert_eq!(rule_sets.len(), 2);
    
    // Verify each program has rules
    for rule_set in &rule_sets {
        assert!(rule_set.rules.len() > 0);
        assert!(!rule_set.program_name.is_empty());
    }
    
    // Verify storage directories were created
    for program in &programs {
        let program_dir = cache_dir.join(&program.name);
        assert!(program_dir.exists());
    }
}

/// Test download progress tracking
#[tokio::test]
async fn test_download_progress_tracking() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();
    
    let mut downloader = BulkRuleDownloader::new(cache_dir).await.unwrap();
    
    let program = AvailableComplianceProgram {
        name: "test_program".to_string(),
        display_name: "Test Program".to_string(),
        description: "Test compliance program".to_string(),
        github_path: "rules/test".to_string(),
        estimated_rule_count: 10,
        category: "Test".to_string(),
        tags: vec!["test".to_string()],
    };
    
    // Create progress tracking
    let progress = Arc::new(Mutex::new(DownloadProgress::new()));
    
    // Download with progress tracking
    let result = downloader.download_with_progress(&program, progress.clone()).await;
    
    assert!(result.is_ok());
    
    // Check final progress state
    let final_progress = progress.lock().await;
    assert_eq!(final_progress.status, DownloadStatus::Completed);
    assert!(final_progress.files_downloaded > 0);
    assert_eq!(final_progress.files_downloaded, final_progress.total_files);
}

/// Test download retry logic for failed downloads
#[tokio::test]
async fn test_download_retry_logic() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();
    
    let mut downloader = BulkRuleDownloader::new(cache_dir).await.unwrap();
    
    // Configure retry settings
    downloader.set_retry_attempts(3);
    downloader.set_retry_delay_ms(100);
    
    let program = AvailableComplianceProgram {
        name: "retry_test".to_string(),
        display_name: "Retry Test".to_string(),
        description: "Test retry logic".to_string(),
        github_path: "rules/nonexistent".to_string(), // This should cause retries
        estimated_rule_count: 5,
        category: "Test".to_string(),
        tags: vec!["test".to_string()],
    };
    
    // Download should handle retries gracefully
    let result = downloader.download_compliance_program_rules(&program).await;
    
    // Should either succeed with placeholder data or fail after retries
    match result {
        Ok(rule_set) => {
            // If successful, should have valid structure
            assert_eq!(rule_set.program_name, "retry_test");
        }
        Err(err) => {
            // If failed, should indicate retry exhaustion
            let error_msg = err.to_string();
            assert!(error_msg.contains("retry") || error_msg.contains("failed") || error_msg.contains("network"));
        }
    }
}

/// Test rule storage and organization
#[tokio::test]
async fn test_rule_storage_organization() {
    let temp_dir = TempDir::new().unwrap();
    let storage_dir = temp_dir.path().to_path_buf();
    
    let mut storage = RuleStorage::new(storage_dir.clone()).await.unwrap();
    
    let rule_set = ComplianceRuleSet {
        program_name: "test_program".to_string(),
        display_name: "Test Program".to_string(),
        version: "1.0.0".to_string(),
        source_url: "https://github.com/test".to_string(),
        download_date: chrono::Utc::now(),
        rules: {
            let mut rules = HashMap::new();
            rules.insert(
                "test_rule_1".to_string(),
                GuardRuleFile {
                    content: "rule test_rule_1 { AWS::S3::Bucket { } }".to_string(),
                    file_path: "test_rule_1.guard".to_string(),
                    last_modified: chrono::Utc::now(),
                }
            );
            rules.insert(
                "test_rule_2".to_string(), 
                GuardRuleFile {
                    content: "rule test_rule_2 { AWS::EC2::Instance { } }".to_string(),
                    file_path: "test_rule_2.guard".to_string(),
                    last_modified: chrono::Utc::now(),
                }
            );
            rules
        },
    };
    
    // Store the rule set
    let result = storage.store_rule_set(&rule_set).await;
    assert!(result.is_ok());
    
    // Verify storage structure
    let program_dir = storage_dir.join("test_program");
    assert!(program_dir.exists());
    
    let rule_file_1 = program_dir.join("test_rule_1.guard");
    let rule_file_2 = program_dir.join("test_rule_2.guard");
    assert!(rule_file_1.exists());
    assert!(rule_file_2.exists());
    
    let metadata_file = program_dir.join("metadata.json");
    assert!(metadata_file.exists());
    
    // Verify file contents
    let rule_content_1 = std::fs::read_to_string(&rule_file_1).unwrap();
    assert!(rule_content_1.contains("AWS::S3::Bucket"));
    
    let rule_content_2 = std::fs::read_to_string(&rule_file_2).unwrap();
    assert!(rule_content_2.contains("AWS::EC2::Instance"));
}

/// Test rule indexing for fast lookup
#[tokio::test]
async fn test_rule_indexing() {
    let temp_dir = TempDir::new().unwrap();
    let storage_dir = temp_dir.path().to_path_buf();
    
    let mut rule_index = RuleIndex::new(storage_dir.clone()).await.unwrap();
    
    let rule_set = ComplianceRuleSet {
        program_name: "indexed_program".to_string(),
        display_name: "Indexed Program".to_string(),
        version: "1.0.0".to_string(),
        source_url: "https://github.com/test".to_string(),
        download_date: chrono::Utc::now(),
        rules: {
            let mut rules = HashMap::new();
            rules.insert(
                "s3_bucket_rule".to_string(),
                GuardRuleFile {
                    content: "rule s3_bucket_rule { AWS::S3::Bucket { Properties { BucketName exists } } }".to_string(),
                    file_path: "s3_bucket_rule.guard".to_string(),
                    last_modified: chrono::Utc::now(),
                }
            );
            rules
        },
    };
    
    // Index the rule set
    let result = rule_index.index_rule_set(&rule_set).await;
    assert!(result.is_ok());
    
    // Test lookups
    let s3_rules = rule_index.find_rules_by_resource_type("AWS::S3::Bucket").await.unwrap();
    assert_eq!(s3_rules.len(), 1);
    assert!(s3_rules.contains(&"s3_bucket_rule".to_string()));
    
    let program_rules = rule_index.find_rules_by_program("indexed_program").await.unwrap();
    assert_eq!(program_rules.len(), 1);
    
    // Test rule content lookup
    let rule_content = rule_index.get_rule_content("s3_bucket_rule").await.unwrap();
    assert!(rule_content.contains("AWS::S3::Bucket"));
}

/// Test rule deduplication across compliance programs
#[tokio::test]
async fn test_rule_deduplication() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();
    
    let mut downloader = BulkRuleDownloader::new(cache_dir).await.unwrap();
    
    // Enable deduplication
    downloader.enable_deduplication(true);
    
    let programs = vec![
        AvailableComplianceProgram {
            name: "program_a".to_string(),
            display_name: "Program A".to_string(),
            description: "First program".to_string(),
            github_path: "rules/program_a".to_string(),
            estimated_rule_count: 10,
            category: "Test".to_string(),
            tags: vec!["test".to_string()],
        },
        AvailableComplianceProgram {
            name: "program_b".to_string(),
            display_name: "Program B".to_string(),
            description: "Second program with overlapping rules".to_string(),
            github_path: "rules/program_b".to_string(),
            estimated_rule_count: 8,
            category: "Test".to_string(),
            tags: vec!["test".to_string()],
        },
    ];
    
    // Download both programs
    let result = downloader.download_multiple_programs(&programs).await;
    assert!(result.is_ok());
    
    let rule_sets = result.unwrap();
    
    // Check that duplicate rules were handled
    // This would be program-specific logic to verify deduplication worked
    assert_eq!(rule_sets.len(), 2);
    
    for rule_set in &rule_sets {
        assert!(rule_set.rules.len() > 0);
    }
}

/// Test partial download recovery
#[tokio::test]
async fn test_partial_download_recovery() {
    let temp_dir = TempDir::new().unwrap();
    let cache_dir = temp_dir.path().to_path_buf();
    
    let mut downloader = BulkRuleDownloader::new(cache_dir.clone()).await.unwrap();
    
    let program = AvailableComplianceProgram {
        name: "partial_recovery_test".to_string(),
        display_name: "Partial Recovery Test".to_string(),
        description: "Test partial download recovery".to_string(),
        github_path: "rules/partial_test".to_string(),
        estimated_rule_count: 20,
        category: "Test".to_string(),
        tags: vec!["test".to_string()],
    };
    
    // Simulate partial download by creating some files
    let program_dir = cache_dir.join(&program.name);
    std::fs::create_dir_all(&program_dir).unwrap();
    std::fs::write(
        program_dir.join("existing_rule.guard"),
        "rule existing_rule { AWS::S3::Bucket { } }"
    ).unwrap();
    
    // Download should detect existing files and continue from where it left off
    let result = downloader.download_compliance_program_rules(&program).await;
    assert!(result.is_ok());
    
    let rule_set = result.unwrap();
    
    // Should include both existing and newly downloaded rules
    assert!(rule_set.rules.len() > 0);
    
    // Existing rule should still be there
    assert!(rule_set.rules.contains_key("existing_rule"));
}

/// Test DownloadProgress structure and updates
#[test]
fn test_download_progress_structure() {
    let mut progress = DownloadProgress::new();
    
    assert_eq!(progress.status, DownloadStatus::NotStarted);
    assert_eq!(progress.files_downloaded, 0);
    assert_eq!(progress.total_files, 0);
    assert_eq!(progress.current_file, None);
    
    // Update progress
    progress.start_download(10);
    assert_eq!(progress.status, DownloadStatus::InProgress);
    assert_eq!(progress.total_files, 10);
    
    progress.update_current_file("test.guard".to_string());
    assert_eq!(progress.current_file, Some("test.guard".to_string()));
    
    progress.increment_downloaded();
    assert_eq!(progress.files_downloaded, 1);
    
    progress.complete_download();
    assert_eq!(progress.status, DownloadStatus::Completed);
}

/// Test error handling for invalid rule content
#[tokio::test]
async fn test_invalid_rule_content_handling() {
    let temp_dir = TempDir::new().unwrap();
    let storage_dir = temp_dir.path().to_path_buf();
    
    let mut storage = RuleStorage::new(storage_dir).await.unwrap();
    
    let invalid_rule_set = ComplianceRuleSet {
        program_name: "invalid_rules".to_string(),
        display_name: "Invalid Rules".to_string(),
        version: "1.0.0".to_string(),
        source_url: "https://github.com/test".to_string(),
        download_date: chrono::Utc::now(),
        rules: {
            let mut rules = HashMap::new();
            rules.insert(
                "invalid_rule".to_string(),
                GuardRuleFile {
                    content: "this is not valid guard syntax!!!".to_string(),
                    file_path: "invalid_rule.guard".to_string(),
                    last_modified: chrono::Utc::now(),
                }
            );
            rules
        },
    };
    
    // Storage should handle invalid content gracefully
    let result = storage.store_rule_set(&invalid_rule_set).await;
    
    // Should either succeed (storing as-is) or provide meaningful error
    match result {
        Ok(_) => {
            // If successful, files should still be stored
            let program_dir = storage.get_storage_dir().join("invalid_rules");
            assert!(program_dir.exists());
        }
        Err(err) => {
            // If failed, should provide helpful error message
            let error_msg = err.to_string();
            assert!(error_msg.contains("invalid") || error_msg.contains("syntax") || error_msg.contains("rule"));
        }
    }
}