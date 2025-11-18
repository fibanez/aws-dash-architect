//! Test file for Agent Framework Debug Logger
//!
//! This test creates a simple example of the debug logging system to verify it works correctly.

#[cfg(test)]
mod tests {
    use super::super::AgentDebugLogger;

    #[test]
    fn test_agent_debug_logger_creation() {
        // Test that the logger can be created successfully
        let logger = AgentDebugLogger::new().expect("Failed to create debug logger");

        // Verify the log path has a parent directory (the logger creates parent dirs)
        assert!(logger.log_path().parent().is_some());

        println!("Debug logger created successfully: {:?}", logger.log_path());
    }
}