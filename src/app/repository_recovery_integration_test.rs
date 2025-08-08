//! Integration tests for repository recovery mechanisms
//!
//! These tests verify that the repository recovery system can detect and recover
//! from various repository corruption scenarios.

#[cfg(test)]
mod tests {
    use super::super::repository_recovery::{
        RepositoryRecoveryManager, RepositoryIssue
    };
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_recovery_manager_detects_missing_directory() {
        let temp_dir = TempDir::new().unwrap();
        let target_path = temp_dir.path().join("nonexistent_repo");
        let required_dirs = vec!["mappings".to_string(), "rules".to_string()];
        
        let manager = RepositoryRecoveryManager::new(
            "https://github.com/test/repo.git".to_string(),
            target_path,
            required_dirs,
        );

        let issues = manager.detect_repository_issues().unwrap();
        assert!(issues.contains(&RepositoryIssue::DirectoryMissing));
        assert!(!issues.is_empty());
    }

    #[test]
    fn test_recovery_manager_detects_empty_repository() {
        let temp_dir = TempDir::new().unwrap();
        let target_path = temp_dir.path().join("empty_repo");
        std::fs::create_dir_all(&target_path).unwrap();
        
        let required_dirs = vec!["mappings".to_string(), "rules".to_string()];
        let manager = RepositoryRecoveryManager::new(
            "https://github.com/test/repo.git".to_string(),
            target_path,
            required_dirs,
        );

        let issues = manager.detect_repository_issues().unwrap();
        assert!(issues.contains(&RepositoryIssue::EmptyRepository));
    }

    #[test]
    fn test_recovery_manager_detects_missing_required_directories() {
        let temp_dir = TempDir::new().unwrap();
        let target_path = temp_dir.path().join("incomplete_repo");
        std::fs::create_dir_all(&target_path).unwrap();
        
        // Create some files but not the required directories
        std::fs::write(target_path.join("README.md"), "test").unwrap();
        
        let required_dirs = vec!["mappings".to_string(), "rules".to_string()];
        let manager = RepositoryRecoveryManager::new(
            "https://github.com/test/repo.git".to_string(),
            target_path,
            required_dirs,
        );

        let issues = manager.detect_repository_issues().unwrap();
        assert!(issues.iter().any(|issue| matches!(issue, RepositoryIssue::MissingRequiredDirectories(_))));
    }

    #[test]
    fn test_repository_validation_comprehensive() {
        let temp_dir = TempDir::new().unwrap();
        let target_path = temp_dir.path().join("test_repo");
        let required_dirs = vec!["mappings".to_string(), "rules".to_string()];
        
        let manager = RepositoryRecoveryManager::new(
            "https://github.com/test/repo.git".to_string(),
            target_path,
            required_dirs,
        );

        let validation = manager.validate_repository_structure();
        assert!(!validation.is_valid); // Should be invalid since directory doesn't exist
        assert!(!validation.validation_errors.is_empty());
        assert_eq!(validation.file_count, 0);
        assert_eq!(validation.directory_count, 0);
    }

    #[test]
    fn test_user_guidance_generation() {
        let temp_dir = TempDir::new().unwrap();
        let target_path = temp_dir.path().join("test_repo");
        let required_dirs = vec!["mappings".to_string(), "rules".to_string()];
        
        let manager = RepositoryRecoveryManager::new(
            "https://github.com/test/repo.git".to_string(),
            target_path,
            required_dirs,
        );

        let issues = vec![
            RepositoryIssue::DirectoryMissing,
            RepositoryIssue::EmptyRepository,
        ];

        let guidance = manager.generate_user_guidance(&issues);
        assert!(guidance.contains("Manual intervention required"));
        assert!(guidance.contains("Repository directory is missing"));
        assert!(guidance.contains("mkdir -p"));
        assert!(guidance.contains("git clone"));
    }

    #[test]
    fn test_recovery_manager_with_valid_repository() {
        let temp_dir = TempDir::new().unwrap();
        let target_path = temp_dir.path().join("valid_repo");
        std::fs::create_dir_all(&target_path).unwrap();
        
        // Create required directories and some files
        std::fs::create_dir_all(target_path.join("mappings")).unwrap();
        std::fs::create_dir_all(target_path.join("rules")).unwrap();
        std::fs::write(target_path.join("README.md"), "test").unwrap();
        std::fs::write(target_path.join("mappings").join("test.json"), "{}").unwrap();
        std::fs::write(target_path.join("rules").join("test.guard"), "rule test {}").unwrap();
        
        let required_dirs = vec!["mappings".to_string(), "rules".to_string()];
        let manager = RepositoryRecoveryManager::new(
            "https://github.com/test/repo.git".to_string(),
            target_path,
            required_dirs,
        );

        let issues = manager.detect_repository_issues().unwrap();
        // Should still have some issues (no .git directory, not openable by git2)
        // but should not have missing directories or empty repository issues
        assert!(!issues.contains(&RepositoryIssue::DirectoryMissing));
        assert!(!issues.contains(&RepositoryIssue::EmptyRepository));
        assert!(!issues.iter().any(|issue| matches!(issue, RepositoryIssue::MissingRequiredDirectories(_))));
    }
}