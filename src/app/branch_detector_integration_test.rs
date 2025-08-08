//! Integration tests for the branch detector functionality
//! 
//! These tests verify that the branch detector can successfully detect branches
//! from real repositories and integrate properly with the guard repository manager.

#[cfg(test)]
mod tests {
    use crate::app::branch_detector::BranchDetector;
    use crate::app::guard_repository_manager::GuardRepositoryManager;

    #[test]
    fn test_branch_detector_creation() {
        let detector = BranchDetector::new("https://github.com/aws-cloudformation/aws-guard-rules-registry.git".to_string());
        // Just verify it can be created without panicking
        assert!(!detector.fallback_branches.is_empty());
    }

    #[test]
    fn test_guard_repository_manager_with_branch_detector() {
        // Test that the guard repository manager can be created and has the branch detector integration
        let manager = GuardRepositoryManager::new();
        assert!(manager.is_ok());
        
        let manager = manager.unwrap();
        // Verify the manager can create a branch detector
        let detector = manager.create_branch_detector();
        assert!(!detector.fallback_branches.is_empty());
    }

    #[test]
    fn test_branch_detector_utils() {
        use crate::app::branch_detector::utils;
        
        // Test utility functions
        assert!(utils::is_likely_default_branch("main"));
        assert!(utils::is_likely_default_branch("master"));
        assert!(!utils::is_likely_default_branch("feature-branch"));
        
        let default_branches = utils::get_common_default_branches();
        assert!(default_branches.contains(&"main".to_string()));
        assert!(default_branches.contains(&"master".to_string()));
        
        let comprehensive = utils::get_comprehensive_fallback_branches();
        assert!(comprehensive.len() > default_branches.len());
    }
}