//! AWS Log Analyzer Agent
//!
//! Specialized agent for CloudWatch logs analysis and troubleshooting.
//! This agent replaces the direct aws_get_log_entries tool with a proper
//! agent-on-demand pattern for better task management and user experience.

use crate::app::bridge::{
    aws_describe_log_groups_tool, aws_get_log_events_tool, 
    aws_find_account_tool, aws_find_region_tool,
    todo_write_tool, todo_read_tool, get_global_aws_credentials,
    PerformanceTimer
};
use crate::time_phase;
use crate::app::bridge::tools::create_agent::{AwsContext, AwsContextSanitized};
use chrono::Utc;
use serde_json;
use stood::agent::Agent;
use stood::telemetry::TelemetryConfig;
use tracing::{debug, info, warn};

/// AWS Log Analyzer Agent - Specialized for CloudWatch logs analysis
pub struct AwsLogAnalyzerAgent;

impl AwsLogAnalyzerAgent {
    /// Create a new AWS Log Analyzer agent with comprehensive CloudWatch toolset
    pub async fn create(
        task_description: String,
        aws_context: AwsContext,
        _agent_id: String, // For future cancellation token integration
    ) -> Result<Agent, stood::StoodError> {
        let mut perf_timer = PerformanceTimer::new("Log Analyzer Agent Creation");
        info!("🔍 Creating AWS Log Analyzer Agent for task: {}", task_description);
        
        let sanitized_context = aws_context.sanitized_for_logging();
        debug!("🔍 AWS context: {:?}", sanitized_context);

        // Create comprehensive system prompt for log analysis
        let system_prompt = time_phase!(perf_timer, "System prompt creation", {
            Self::create_system_prompt(&task_description, &sanitized_context)
        });
        
        // Configure telemetry for the log analyzer agent
        let mut telemetry_config = time_phase!(perf_timer, "Telemetry configuration", {
            TelemetryConfig::default()
                .with_service_name("aws-log-analyzer-agent")
                .with_service_version("1.0.0")
                .with_otlp_endpoint("http://localhost:4320") // HTTP OTLP endpoint
                .with_batch_processing()
        });

        // Enable debug tracing for detailed log analysis tracking
        telemetry_config.enable_debug_tracing = true;
        telemetry_config.service_attributes.insert(
            "agent.type".to_string(), 
            "aws-log-analyzer".to_string()
        );
        telemetry_config.service_attributes.insert(
            "agent.task".to_string(), 
            task_description.clone()
        );
        telemetry_config.service_attributes.insert(
            "aws.account_id".to_string(), 
            sanitized_context.account_id.clone()
        );
        telemetry_config.service_attributes.insert(
            "aws.region".to_string(), 
            sanitized_context.region.clone()
        );
        telemetry_config.service_attributes.insert(
            "aws.resource".to_string(), 
            sanitized_context.resource_identifier.clone()
        );

        // Add unique session identifier for this log analyzer instance
        let session_id = format!("log-analyzer-{}", Utc::now().timestamp_millis());
        telemetry_config.service_attributes.insert("session.id".to_string(), session_id);

        // Build the specialized log analyzer agent with timing
        let mut agent_builder = time_phase!(perf_timer, "Agent builder setup", {
            Agent::builder()
                .system_prompt(system_prompt)
                .with_telemetry(telemetry_config)
                .tools(vec![
                    // Task management tools for progress tracking
                    todo_write_tool(),
                    todo_read_tool(),
                    
                    // AWS CloudWatch tools for log analysis
                    aws_describe_log_groups_tool(None), // Find relevant log groups
                    aws_get_log_events_tool(None),      // Retrieve log events with filtering
                    
                    // AWS context tools (no API calls)
                    aws_find_account_tool(),
                    aws_find_region_tool(),
                ])
        });

        // Add AWS credentials if available globally (same pattern as main Bridge Agent)
        time_phase!(perf_timer, "Credential configuration", {
            if let Some((access_key, secret_key, session_token, region)) = get_global_aws_credentials() {
                info!("🔐 Using global AWS credentials for log analyzer agent");
                agent_builder = agent_builder.with_credentials(access_key, secret_key, session_token, region);
            } else {
                warn!("⚠️ No global AWS credentials available for log analyzer agent - using default credential chain");
            }
        });

        let agent = time_phase!(perf_timer, "Agent.build() - CRITICAL TIMING", {
            agent_builder.build().await?
        });

        perf_timer.complete();
        info!("✅ AWS Log Analyzer Agent created successfully");
        Ok(agent)
    }

    /// Create comprehensive system prompt for log analysis tasks
    fn create_system_prompt(task_description: &str, context: &AwsContextSanitized) -> String {
        format!(r#"You are an AWS CloudWatch logs analysis specialist.

IMPORTANT: You MUST use the TodoWrite tool to track your progress through the CloudWatch log analysis process.

DO NOT proceed without the provided AWS context: account_id, region, and resource_identifier.

CRITICAL LOG ANALYSIS WORKFLOW:
1. Use TodoWrite to plan your analysis: ["Find relevant log groups", "Identify log streams", "Retrieve log events", "Analyze patterns and errors", "Summarize findings"]
2. aws_describe_log_groups - Find log groups related to the resource
3. aws_get_log_events - Retrieve actual log data with appropriate filtering
4. Analyze patterns, errors, and anomalies in the logs
5. Mark todos complete as you progress through each step

SECURITY GUIDELINES:
- NEVER expose AWS credentials, keys, or tokens in responses
- DO NOT log sensitive information from AWS resources
- Focus on error patterns and troubleshooting insights
- Sanitize any sensitive data before including in responses

LOG ANALYSIS BEST PRACTICES:
- Use time range filtering to focus on relevant periods
- Apply filter patterns to identify specific error types
- Look for error patterns, exceptions, and anomalies
- Provide actionable troubleshooting recommendations
- Correlate events across different log streams when possible

AVAILABLE TOOLS:
- TodoWrite: Track your analysis progress (USE THIS FIRST)
- TodoRead: Check current task status
- aws_describe_log_groups: Find CloudWatch log groups
- aws_get_log_events: Retrieve log events with filtering and time ranges
- aws_find_account: Account lookup (no API calls)
- aws_find_region: Region lookup (no API calls)

CURRENT ANALYSIS TASK: {task_description}

AWS CONTEXT:
- Account ID: {account_id}
- Region: {region}
- Resource: {resource_identifier}

EXPECTED OUTPUT:
Provide clear, actionable insights about the logs including:
- Error patterns and their frequency
- Potential root causes
- Recommended troubleshooting steps
- Timeline of events if relevant
- Correlation with AWS resource health if applicable

Remember to use TodoWrite at the beginning to break down your analysis approach."#,
            task_description = task_description,
            account_id = context.account_id,
            region = context.region,
            resource_identifier = context.resource_identifier
        )
    }

    /// Execute a log analysis task with the created agent
    pub async fn execute_analysis(
        agent: &mut Agent,
        task_description: &str,
    ) -> Result<serde_json::Value, stood::StoodError> {
        info!("🔍 Executing log analysis: {}", task_description);
        
        // Execute the analysis task
        let result = agent.execute(task_description).await?;
        
        info!("✅ Log analysis completed successfully");
        debug!("Analysis result: {} chars", result.response.len());

        // Return structured result
        Ok(serde_json::json!({
            "analysis_type": "aws-log-analyzer",
            "task_completed": true,
            "response": result.response,
            "execution_summary": {
                "cycles": result.execution.cycles,
                "model_calls": result.execution.model_calls,
                "tool_executions": result.execution.tool_executions,
                "used_tools": result.used_tools,
                "success": result.success
            },
            "timestamp": Utc::now().to_rfc3339()
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_aws_context() -> AwsContext {
        AwsContext {
            account_id: "123456789012".to_string(),
            region: "us-east-1".to_string(),
            resource_identifier: "my-lambda-function".to_string(),
            access_key: "AKIA...".to_string(),
            secret_key: "secret...".to_string(),
            session_token: Some("token...".to_string()),
        }
    }

    #[test]
    fn test_system_prompt_creation() {
        let context = create_test_aws_context().sanitized_for_logging();
        let prompt = AwsLogAnalyzerAgent::create_system_prompt(
            "Find errors in Lambda function logs", 
            &context
        );
        
        assert!(prompt.contains("CloudWatch logs analysis specialist"));
        assert!(prompt.contains("123456789012"));
        assert!(prompt.contains("us-east-1"));
        assert!(prompt.contains("my-lambda-function"));
        assert!(prompt.contains("TodoWrite"));
        assert!(prompt.contains("aws_describe_log_groups"));
        assert!(prompt.contains("SECURITY GUIDELINES"));
    }

    #[tokio::test]
    async fn test_agent_creation_components() {
        let aws_context = create_test_aws_context();
        let task = "Analyze Lambda function errors".to_string();
        let agent_id = "test-agent-123".to_string();
        
        // Test that we can create the components without actually building the agent
        // (since we need real AWS credentials for that)
        let sanitized = aws_context.sanitized_for_logging();
        let prompt = AwsLogAnalyzerAgent::create_system_prompt(&task, &sanitized);
        
        assert!(!prompt.is_empty());
        assert!(prompt.contains("aws-describe-log-groups"));
        assert!(prompt.contains("TodoWrite"));
    }
}