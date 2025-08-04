//! AWS Resource Auditor Agent
//!
//! Specialized agent for comprehensive resource inventory and compliance checking.
//! This agent provides detailed analysis of AWS resources including relationships,
//! tagging compliance, and utilization assessment.

use crate::app::bridge::{
    aws_list_resources_tool, aws_describe_resource_tool,
    aws_find_account_tool, aws_find_region_tool,
    todo_write_tool, todo_read_tool, get_global_aws_credentials
};
use crate::app::bridge::tools::create_agent::{AwsContext, AwsContextSanitized};
use chrono::Utc;
use serde_json;
use stood::agent::Agent;
use stood::telemetry::TelemetryConfig;
use tracing::{debug, info, warn};

/// AWS Resource Auditor Agent - Specialized for resource inventory and compliance
pub struct AwsResourceAuditorAgent;

impl AwsResourceAuditorAgent {
    /// Create a new AWS Resource Auditor agent with comprehensive resource tools
    pub async fn create(
        task_description: String,
        aws_context: AwsContext,
        _agent_id: String, // For future cancellation token integration
    ) -> Result<Agent, stood::StoodError> {
        info!("📊 Creating AWS Resource Auditor Agent for task: {}", task_description);
        
        let sanitized_context = aws_context.sanitized_for_logging();
        debug!("📊 AWS context: {:?}", sanitized_context);

        // Create comprehensive system prompt for resource auditing
        let system_prompt = Self::create_system_prompt(&task_description, &sanitized_context);
        
        // Configure telemetry for the resource auditor agent
        let mut telemetry_config = TelemetryConfig::default()
            .with_service_name("aws-resource-auditor-agent")
            .with_service_version("1.0.0")
            .with_otlp_endpoint("http://localhost:4320") // HTTP OTLP endpoint
            .with_batch_processing();

        // Enable debug tracing for detailed resource audit tracking
        telemetry_config.enable_debug_tracing = true;
        telemetry_config.service_attributes.insert(
            "agent.type".to_string(), 
            "aws-resource-auditor".to_string()
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

        // Add unique session identifier for this auditor instance
        let session_id = format!("resource-auditor-{}", Utc::now().timestamp_millis());
        telemetry_config.service_attributes.insert("session.id".to_string(), session_id);

        // Build the specialized resource auditor agent
        let mut agent_builder = Agent::builder()
            .system_prompt(system_prompt)
            .with_telemetry(telemetry_config)
            .tools(vec![
                // Task management tools for progress tracking
                todo_write_tool(),
                todo_read_tool(),
                
                // AWS resource discovery and analysis tools
                aws_list_resources_tool(None),     // Discover AWS resources
                aws_describe_resource_tool(None),  // Get detailed resource information
                
                // AWS context tools (no API calls)
                aws_find_account_tool(),
                aws_find_region_tool(),
            ]);

        // Add AWS credentials if available globally (same pattern as main Bridge Agent)
        if let Some((access_key, secret_key, session_token, region)) = get_global_aws_credentials() {
            info!("🔐 Using global AWS credentials for resource auditor agent");
            agent_builder = agent_builder.with_credentials(access_key, secret_key, session_token, region);
        } else {
            warn!("⚠️ No global AWS credentials available for resource auditor agent - using default credential chain");
        }

        let agent = agent_builder.build().await?;

        info!("✅ AWS Resource Auditor Agent created successfully");
        Ok(agent)
    }

    /// Create comprehensive system prompt for resource auditing tasks
    fn create_system_prompt(task_description: &str, context: &AwsContextSanitized) -> String {
        format!(r#"You are an AWS resource auditing specialist.

IMPORTANT: Use TodoWrite to break down complex auditing tasks and track your progress.

DO NOT proceed without proper AWS context (account_id, region).

COMPREHENSIVE AUDIT WORKFLOW:
1. Use TodoWrite to plan audit scope: ["Define audit scope", "Discover resources", "Analyze configurations", "Check compliance", "Generate report"]
2. aws_list_resources to discover AWS resources in scope
3. aws_describe_resource for detailed analysis of each resource type
4. Analyze resource configurations, relationships, and compliance
5. Mark todos complete as you progress through the audit

AUDIT CAPABILITIES:
- Comprehensive resource inventory across AWS services
- Resource relationship mapping and dependency analysis
- Compliance and tagging analysis
- Resource utilization assessment
- Cost optimization opportunities identification
- Security posture evaluation (configurations only)

SECURITY GUIDELINES:
- NEVER expose sensitive resource configurations unnecessarily
- DO NOT log AWS credentials, keys, or tokens
- Focus on configuration analysis and compliance
- Sanitize sensitive data before including in responses
- Provide security recommendations based on best practices

AUDIT FOCUS AREAS:
- Resource tagging compliance and consistency
- Security group configurations and access patterns
- IAM policies and permissions (read-only analysis)
- Resource utilization and optimization opportunities
- Backup and disaster recovery configurations
- Network architecture and connectivity
- Cost allocation and resource organization

AVAILABLE TOOLS:
- TodoWrite: Track audit progress (USE THIS FIRST)
- TodoRead: Check audit status
- aws_list_resources: Discover AWS resources with filtering
- aws_describe_resource: Get detailed resource information
- aws_find_account: Account lookup (no API calls)
- aws_find_region: Region lookup (no API calls)

CURRENT AUDIT TASK: {task_description}

AWS CONTEXT:
- Account ID: {account_id}
- Region: {region}

EXPECTED OUTPUT:
Provide structured audit findings including:
- Resource inventory summary
- Compliance status and gaps
- Security configuration analysis
- Cost optimization recommendations
- Tagging consistency report
- Resource relationship mapping
- Actionable remediation steps

AUDIT REPORT STRUCTURE:
1. Executive Summary
2. Resource Inventory
3. Compliance Findings
4. Security Analysis
5. Cost Optimization Opportunities
6. Recommendations and Next Steps

Remember to use TodoWrite at the beginning to outline your audit approach and track progress through each phase."#,
            task_description = task_description,
            account_id = context.account_id,
            region = context.region
        )
    }

    /// Execute a resource audit task with the created agent
    pub async fn execute_audit(
        agent: &mut Agent,
        task_description: &str,
    ) -> Result<serde_json::Value, stood::StoodError> {
        info!("📊 Executing resource audit: {}", task_description);
        
        // Execute the audit task
        let result = agent.execute(task_description).await?;
        
        info!("✅ Resource audit completed successfully");
        debug!("Audit result: {} chars", result.response.len());

        // Return structured result
        Ok(serde_json::json!({
            "audit_type": "aws-resource-auditor",
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
            region: "us-west-2".to_string(),
            resource_identifier: "audit-scope".to_string(),
            access_key: "AKIA...".to_string(),
            secret_key: "secret...".to_string(),
            session_token: Some("token...".to_string()),
        }
    }

    #[test]
    fn test_system_prompt_creation() {
        let context = create_test_aws_context().sanitized_for_logging();
        let prompt = AwsResourceAuditorAgent::create_system_prompt(
            "Audit EC2 instances for compliance", 
            &context
        );
        
        assert!(prompt.contains("resource auditing specialist"));
        assert!(prompt.contains("123456789012"));
        assert!(prompt.contains("us-west-2"));
        assert!(prompt.contains("TodoWrite"));
        assert!(prompt.contains("aws_list_resources"));
        assert!(prompt.contains("AUDIT CAPABILITIES"));
        assert!(prompt.contains("COMPLIANCE"));
    }

    #[tokio::test]
    async fn test_audit_components() {
        let aws_context = create_test_aws_context();
        let task = "Comprehensive security audit".to_string();
        let agent_id = "audit-agent-456".to_string();
        
        let sanitized = aws_context.sanitized_for_logging();
        let prompt = AwsResourceAuditorAgent::create_system_prompt(&task, &sanitized);
        
        assert!(!prompt.is_empty());
        assert!(prompt.contains("aws_describe_resource"));
        assert!(prompt.contains("Security configuration analysis"));
        assert!(prompt.contains("TodoWrite"));
    }
}