//! Tests for the git error handling system

#[cfg(test)]
mod tests {
    use super::git_error_handling::*;
    use std::path::PathBuf;

    #[test]
    fn test_platform_info_creation() {
        let path = PathBuf::from("/Users/test user/Documents/my repo");
        let platform_info = PlatformInfo::from_path(&path);
        
        assert_eq!(platform_info.os, std::env::consts::OS);
        assert!(platform_info.path_contains_spaces);
        assert!(!platform_info.path_contains_special_chars);
        assert!(platform_info.path_length > 0);
        assert!(!platform_info.path_components.is_empty());
    }

    #[test]
    fn test_git_operation_error_creation() {
        let path = PathBuf::from("/test/path");
        let error = GitOperationError::new(
            GitOperation::Clone,
            "Test error".to_string(),
            path.clone(),
            Some("https://github.com/test/repo.git".to_string()),
            Some("main".to_string()),
        );

        assert_eq!(error.operation, GitOperation::Clone);
        assert_eq!(error.path, path);
        assert!(error.repository_url.is_some());
        assert!(error.branch_name.is_some());
        assert!(error.additional_context.contains_key("error_message"));
    }

    #[test]
    fn test_git_operation_logger_creation() {
        let path = PathBuf::from("/test/repo");
        let logger = GitOperationLogger::start(
            GitOperation::Clone,
            path.clone(),
            Some("https://github.com/test/repo.git".to_string()),
            Some("main".to_string()),
        );

        assert_eq!(logger.operation, GitOperation::Clone);
        assert_eq!(logger.path, path);
        assert!(logger.repository_url.is_some());
        assert!(logger.branch_name.is_some());
    }

    #[test]
    fn test_platform_info_special_characters() {
        let path = PathBuf::from("/test/path with spaces & special chars!");
        let platform_info = PlatformInfo::from_path(&path);
        
        assert!(platform_info.path_contains_spaces);
        assert!(platform_info.path_contains_special_chars);
    }

    #[test]
    fn test_git_operation_error_with_context() {
        let path = PathBuf::from("/test/path");
        let error = GitOperationError::new(
            GitOperation::Clone,
            "Test error".to_string(),
            path,
            None,
            None,
        ).with_context("test_key".to_string(), "test_value".to_string());

        assert!(error.additional_context.contains_key("test_key"));
        assert_eq!(error.additional_context.get("test_key"), Some(&"test_value".to_string()));
    }
}