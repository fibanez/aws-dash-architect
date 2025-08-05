//! Generic Task Agent
//!
//! Flexible agent that can handle any AWS task based on natural language descriptions.
//! Replaces hardcoded agent types with dynamic task-based agent creation.

use crate::app::bridge::{
    aws_describe_log_groups_tool, aws_get_log_events_tool, 
    aws_list_resources_tool, aws_describe_resource_tool,
    aws_find_account_tool, aws_find_region_tool,
    todo_write_tool, todo_read_tool, get_global_aws_credentials,
    PerformanceTimer, SubAgentCallbackHandler, get_global_bridge_sender,
    get_global_model, ModelConfig
};
use crate::time_phase;
use chrono::Utc;
use serde_json;
use stood::agent::Agent;
use stood::telemetry::TelemetryConfig;
use tracing::{debug, info, warn};

/// Generic Task Agent - Handles any AWS task based on description
pub struct TaskAgent;

impl TaskAgent {
    /// Create a new generic task agent with comprehensive AWS toolset
    pub async fn create(
        task_id: String,
        task_description: String,
        account_ids: Vec<String>,
        regions: Vec<String>,
        model_id: Option<String>,
    ) -> Result<Agent, stood::StoodError> {
        let mut perf_timer = PerformanceTimer::new("Generic Task Agent Creation");
        info!("🎯 Creating Generic Task Agent for: {}", task_description);
        
        debug!("🎯 Task context - Accounts: {:?}, Regions: {:?}", account_ids, regions);

        // Determine model to use (parameter > global > default)
        let selected_model = model_id
            .or_else(|| get_global_model())
            .unwrap_or_else(|| ModelConfig::default_model_id());
        
        info!("🤖 Task agent will use model: {}", selected_model);

        // Create dynamic system prompt based on task description
        let system_prompt = time_phase!(perf_timer, "System prompt creation", {
            Self::create_system_prompt(&task_description, &account_ids, &regions)
        });
        
        // Configure telemetry for the task agent
        let mut telemetry_config = time_phase!(perf_timer, "Telemetry configuration", {
            TelemetryConfig::default()
                .with_service_name("aws-task-agent")
                .with_service_version("1.0.0")
                .with_otlp_endpoint("http://localhost:4320") // HTTP OTLP endpoint
                .with_batch_processing()
        });

        // Enable debug tracing for detailed task tracking
        telemetry_config.enable_debug_tracing = true;
        telemetry_config.service_attributes.insert(
            "agent.type".to_string(), 
            "generic-task-agent".to_string()
        );
        telemetry_config.service_attributes.insert(
            "task.description".to_string(), 
            task_description.clone()
        );
        telemetry_config.service_attributes.insert(
            "aws.account_ids".to_string(), 
            account_ids.join(",")
        );
        telemetry_config.service_attributes.insert(
            "aws.regions".to_string(), 
            regions.join(",")
        );
        telemetry_config.service_attributes.insert(
            "task.id".to_string(), 
            task_id.clone()
        );

        // Add unique session identifier for this task agent instance
        let session_id = format!("task-agent-{}", Utc::now().timestamp_millis());
        telemetry_config.service_attributes.insert("session.id".to_string(), session_id);

        // Build the generic task agent with all AWS tools
        let mut agent_builder = time_phase!(perf_timer, "Agent builder setup", {
            // TODO: Add model configuration once stood library model API is clarified
            // let model = ModelConfig::id_to_bedrock_model(&selected_model);
            let mut builder = Agent::builder()
                // .model(model)  // TODO: Re-enable when model API is available
                .system_prompt(system_prompt)
                .with_telemetry(telemetry_config)
                .with_think_tool("Think carefully about what we need to do next")
                .tools(vec![
                    // Task management tools for progress tracking
                    todo_write_tool(),
                    todo_read_tool(),
                    
                    // AWS CloudWatch tools for log analysis
                    aws_describe_log_groups_tool(None),
                    aws_get_log_events_tool(None),
                    
                    // AWS resource tools for resource operations
                    aws_list_resources_tool(None),
                    aws_describe_resource_tool(None),
                    
                    // AWS context tools (no API calls)
                    aws_find_account_tool(),
                    aws_find_region_tool(),
                ]);

            // Add callback handler for event bubbling to Bridge UI
            if let Some(bridge_sender) = get_global_bridge_sender() {
                info!("📡 Task agent using Bridge event bubbling with user-friendly language");
                builder = builder.with_callback_handler(
                    SubAgentCallbackHandler::with_sender(
                        task_id.clone(),
                        "generic-task-agent".to_string(),
                        bridge_sender,
                    ),
                );
            } else {
                info!("📊 Task agent without Bridge event bubbling (standalone mode)");
                builder = builder.with_callback_handler(
                    SubAgentCallbackHandler::new(task_id.clone(), "generic-task-agent".to_string()),
                );
            }

            builder
        });

        // Add AWS credentials if available globally (same pattern as specialized agents)
        time_phase!(perf_timer, "Credential configuration", {
            if let Some((access_key, secret_key, session_token, region_creds)) = get_global_aws_credentials() {
                info!("🔐 Using global AWS credentials for task agent");
                agent_builder = agent_builder.with_credentials(access_key, secret_key, session_token, region_creds);
            } else {
                warn!("⚠️ No global AWS credentials available for task agent - using default credential chain");
            }
        });

        let agent = time_phase!(perf_timer, "Agent.build() - CRITICAL TIMING", {
            agent_builder.build().await?
        });

        perf_timer.complete();
        info!("✅ Generic Task Agent created successfully");
        Ok(agent)
    }

    /// Create dynamic system prompt based on task description and context
    fn create_system_prompt(task_description: &str, account_ids: &[String], regions: &[String]) -> String {
        let accounts_text = if account_ids.len() == 1 {
            format!("- Account ID: {}", account_ids[0])
        } else {
            format!("- Account IDs: {}", account_ids.join(", "))
        };
        
        let regions_text = if regions.len() == 1 {
            format!("- Region: {}", regions[0])
        } else {
            format!("- Regions: {}", regions.join(", "))
        };
        
        format!(r#"You are an AWS task specialist. Execute this specific task: {}

AWS Context:
{}
{}

IMPORTANT: You MUST use the TodoWrite tool to track your progress through this task.

TASK EXECUTION WORKFLOW:
1. Use TodoWrite to plan your approach: break down the task into specific steps
2. Use appropriate AWS tools based on what the task requires:
   - CloudWatch logs: aws_describe_log_groups, aws_get_log_events
   - AWS resources: aws_list_resources, aws_describe_resource (supports multiple accounts/regions)
   - Context lookup: aws_find_account, aws_find_region
3. Execute the steps systematically, marking todos complete as you progress
4. Provide a comprehensive summary of your findings

MULTI-ACCOUNT/REGION OPERATIONS:
- When working with multiple accounts or regions, process them systematically
- Use the aws_list_resources tool which accepts arrays of account IDs and regions
- Aggregate findings across all specified accounts and regions
- Clearly organize results by account/region when reporting

SECURITY GUIDELINES:
- NEVER expose AWS credentials, keys, or tokens in responses
- DO NOT log sensitive information from AWS resources
- Focus on the specific task requested
- Sanitize any sensitive data before including in responses

AVAILABLE TOOLS:
- TodoWrite: Track your task progress (USE THIS FIRST)
- TodoRead: Check current task status
- aws_describe_log_groups: Find CloudWatch log groups
- aws_get_log_events: Retrieve log events with filtering
- aws_list_resources: List AWS resources by type (supports arrays of accounts/regions)
- aws_describe_resource: Get detailed resource information
- aws_find_account: Account lookup (no API calls)
- aws_find_region: Region lookup (no API calls)

EXPECTED OUTPUT:
Complete the task efficiently and provide:
- Clear findings and insights
- Actionable recommendations if applicable
- Summary of what was accomplished
- Any important observations or patterns discovered

Remember to use TodoWrite at the beginning to organize your approach to this task."#,
            task_description, accounts_text, regions_text
        )
    }

    /// Execute a task with the created agent
    pub async fn execute_task(
        agent: &mut Agent,
        task_description: &str,
    ) -> Result<serde_json::Value, stood::StoodError> {
        info!("🎯 Executing task: {}", task_description);
        
        // Execute the task
        let result = agent.execute(task_description).await?;
        
        info!("✅ Task completed successfully");
        debug!("Task result: {} chars", result.response.len());

        // Return structured result
        Ok(serde_json::json!({
            "task_type": "generic-task-agent",
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

    #[test]
    fn test_system_prompt_creation() {
        let prompt = TaskAgent::create_system_prompt(
            "Analyze Lambda function performance", 
            &vec!["123456789012".to_string()],
            &vec!["us-east-1".to_string()]
        );
        
        assert!(prompt.contains("Analyze Lambda function performance"));
        assert!(prompt.contains("123456789012"));
        assert!(prompt.contains("us-east-1"));
        assert!(prompt.contains("TodoWrite"));
        assert!(prompt.contains("aws_describe_log_groups"));
        assert!(prompt.contains("SECURITY GUIDELINES"));
    }

    #[test]
    fn test_system_prompt_creation_multiple_accounts_regions() {
        let prompt = TaskAgent::create_system_prompt(
            "Analyze multi-region Lambda performance", 
            &vec!["123456789012".to_string(), "123456789013".to_string()],
            &vec!["us-east-1".to_string(), "eu-west-1".to_string()]
        );
        
        assert!(prompt.contains("Analyze multi-region Lambda performance"));
        assert!(prompt.contains("Account IDs: 123456789012, 123456789013"));
        assert!(prompt.contains("Regions: us-east-1, eu-west-1"));
        assert!(prompt.contains("MULTI-ACCOUNT/REGION OPERATIONS"));
        assert!(prompt.contains("supports arrays of account IDs and regions"));
    }

    #[tokio::test]
    async fn test_agent_creation_components() {
        let _task_id = "test-task-123".to_string();
        let task_description = "Test AWS resource analysis".to_string();
        let account_ids = vec!["123456789012".to_string()];
        let regions = vec!["us-east-1".to_string()];
        
        // Test that we can create the components without actually building the agent
        // (since we need real AWS credentials for that)
        let prompt = TaskAgent::create_system_prompt(&task_description, &account_ids, &regions);
        
        assert!(!prompt.is_empty());
        assert!(prompt.contains("Test AWS resource analysis"));
        assert!(prompt.contains("TodoWrite"));
    }
}