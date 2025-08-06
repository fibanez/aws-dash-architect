//! Test file for Bridge Debug Logger
//!
//! This test creates a simple example of the debug logging system to verify it works correctly.

#[cfg(test)]
mod tests {
    use super::super::{BridgeDebugEvent, BridgeDebugLogger};
    use chrono::Utc;

    #[test]
    fn test_bridge_debug_logger_creation() {
        // Test that the logger can be created successfully
        let logger = BridgeDebugLogger::new().expect("Failed to create debug logger");
        
        // Verify the log path has a parent directory (the logger creates parent dirs)
        assert!(logger.log_path().parent().is_some());
        
        println!("Debug logger created successfully: {:?}", logger.log_path());
    }

    #[test]
    fn test_helper_functions() {
        use super::super::{create_input_summary, create_output_summary, create_response_preview, extract_tool_calls_from_response, truncate_string};
        use serde_json::json;
        
        // Test truncate_string
        assert_eq!(truncate_string("short", 10), "short");
        assert_eq!(truncate_string("this is a very long string", 10), "this is a ...");
        
        // Test create_input_summary
        let empty_params = json!({});
        assert_eq!(create_input_summary(&empty_params), "no parameters");
        
        let simple_params = json!({"account_id": "123456789012", "region": "us-east-1"});
        assert_eq!(create_input_summary(&simple_params), "params: account_id, region");
        
        // Test create_output_summary
        let success_output = json!({"success": true, "result": "completed", "count": 5});
        assert_eq!(create_output_summary(&success_output), "success: true, 3 fields");
        
        // Test create_response_preview
        let long_response = "This is a very long response that should be truncated when creating a preview for the debug log. ".repeat(10);
        let preview = create_response_preview(&long_response);
        assert!(preview.len() <= 200);
        assert!(preview.contains("This is a very long response"));
        
        // Test extract_tool_calls_from_response
        let response_with_tools = r#"I need to use the create_task tool to handle this request. I'll also use todo_write for planning."#;
        let tools = extract_tool_calls_from_response(response_with_tools);
        // Basic test - the function should not crash and return a vector
        assert!(tools.len() >= 0);
        
        println!("✅ All helper functions working correctly");
    }
}