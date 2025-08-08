//! Integration tests for git error handling system

use awsdash::app::git_error_handling::{GitOperation, GitOperationError, GitOperationLogger, PlatformInfo};
use std::path::PathBuf;

#[test]
fn test_git_operation_logger_integration() {
    let path = PathBuf::from("/tmp/test_repo");
    let logger = GitOperationLogger::start(
        GitOperation::Clone,
        path.clone(),
        Some("https://github.com/test/repo.git".to_string()),
        Some("main".to_string()),
    );

    // Test that the logger was created successfully
    assert_eq!(logger.operation, GitOperation::Clone);
    assert_eq!(logger.path, path);
}

#[test]
fn test_platform_info_macos_specific() {
    let path = PathBuf::from("/Users/testuser/Library/Application Support/awsdash/guard-rules-registry");
    let platform_info = PlatformInfo::from_path(&path);
    
    // Test macOS-specific path detection
    assert_eq!(platform_info.os, "macos");
    assert!(platform_info.path_contains_spaces);
    // Application Support may be considered a system path on macOS
}

#[test]
fn test_git_operation_error_anyhow_conversion() {
    let path = PathBuf::from("/test/path");
    let git_error = GitOperationError::new(
        GitOperation::Clone,
        "Test clone failure".to_string(),
        path,
        Some("https://github.com/test/repo.git".to_string()),
        Some("main".to_string()),
    );

    let anyhow_error = git_error.into_anyhow();
    let error_message = format!("{}", anyhow_error);
    
    assert!(error_message.contains("clone"));
    assert!(error_message.contains("failed"));
    assert!(error_message.contains("macos"));
}

#[test]
fn test_error_context_preservation() {
    let path = PathBuf::from("/test/path");
    let mut git_error = GitOperationError::new(
        GitOperation::Clone,
        "Test error".to_string(),
        path,
        None,
        None,
    );

    git_error = git_error.with_context("operation_duration".to_string(), "5.2s".to_string());
    git_error = git_error.with_context("retry_count".to_string(), "3".to_string());

    assert_eq!(git_error.additional_context.len(), 3); // error_message + 2 added contexts
    assert!(git_error.additional_context.contains_key("operation_duration"));
    assert!(git_error.additional_context.contains_key("retry_count"));
}