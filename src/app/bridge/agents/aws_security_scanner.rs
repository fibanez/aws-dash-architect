//! AWS Security Scanner Agent
//!
//! Specialized agent for defensive security posture assessment and vulnerability scanning.
//! This agent performs READ-ONLY security analysis and provides defensive recommendations.
//! NEVER performs actions that could compromise security or modify security configurations.

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

/// AWS Security Scanner Agent - Specialized for DEFENSIVE security analysis
pub struct AwsSecurityScannerAgent;

impl AwsSecurityScannerAgent {
    /// Create a new AWS Security Scanner agent with defensive security tools
    pub async fn create(
        task_description: String,
        aws_context: AwsContext,
        _agent_id: String, // For future cancellation token integration
    ) -> Result<Agent, stood::StoodError> {
        info!("🔒 Creating AWS Security Scanner Agent for task: {}", task_description);
        
        let sanitized_context = aws_context.sanitized_for_logging();
        debug!("🔒 AWS context: {:?}", sanitized_context);

        // Create comprehensive system prompt for security scanning
        let system_prompt = Self::create_system_prompt(&task_description, &sanitized_context);
        
        // Configure telemetry for the security scanner agent
        let mut telemetry_config = TelemetryConfig::default()
            .with_service_name("aws-security-scanner-agent")
            .with_service_version("1.0.0")
            .with_otlp_endpoint("http://localhost:4320") // HTTP OTLP endpoint
            .with_batch_processing();

        // Enable debug tracing for detailed security scan tracking
        telemetry_config.enable_debug_tracing = true;
        telemetry_config.service_attributes.insert(
            "agent.type".to_string(), 
            "aws-security-scanner".to_string()
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
            "security.mode".to_string(), 
            "defensive-readonly".to_string()
        );

        // Add unique session identifier for this security scanner instance
        let session_id = format!("security-scanner-{}", Utc::now().timestamp_millis());
        telemetry_config.service_attributes.insert("session.id".to_string(), session_id);

        // Build the specialized security scanner agent
        let mut agent_builder = Agent::builder()
            .system_prompt(system_prompt)
            .with_telemetry(telemetry_config)
            .tools(vec![
                // Task management tools for progress tracking
                todo_write_tool(),
                todo_read_tool(),
                
                // AWS resource discovery tools (READ-ONLY)
                aws_list_resources_tool(None),     // Discover resources for security review
                aws_describe_resource_tool(None),  // Analyze resource security posture
                
                // AWS context tools (no API calls)
                aws_find_account_tool(),
                aws_find_region_tool(),
            ]);

        // Add AWS credentials if available globally (same pattern as main Bridge Agent)
        if let Some((access_key, secret_key, session_token, region)) = get_global_aws_credentials() {
            info!("🔐 Using global AWS credentials for security scanner agent");
            agent_builder = agent_builder.with_credentials(access_key, secret_key, session_token, region);
        } else {
            warn!("⚠️ No global AWS credentials available for security scanner agent - using default credential chain");
        }

        let agent = agent_builder.build().await?;

        info!("✅ AWS Security Scanner Agent created successfully");
        Ok(agent)
    }

    /// Create comprehensive system prompt for defensive security scanning
    fn create_system_prompt(task_description: &str, context: &AwsContextSanitized) -> String {
        format!(r#"You are an AWS security scanning specialist.

CRITICAL SECURITY RULES:
- NEVER log, display, or expose AWS credentials, keys, tokens, or secrets
- NEVER suggest actions that could compromise security
- NEVER bypass AWS security controls or policies
- DO NOT disable security features without explicit justification
- NEVER perform destructive or modifying operations

IMPORTANT: This agent performs DEFENSIVE security analysis ONLY.

SECURITY ASSESSMENT WORKFLOW:
1. Use TodoWrite to plan security assessment: ["Define security scope", "Discover resources", "Analyze security configurations", "Identify vulnerabilities", "Provide recommendations"]
2. aws_list_resources to discover resources for security review
3. aws_describe_resource to analyze resource security posture
4. Evaluate configurations against security best practices
5. Mark todos complete as you progress through the assessment

DEFENSIVE SECURITY CAPABILITIES:
- Security group analysis (not modification)
- IAM policy review (not creation/modification)
- Resource exposure assessment
- Network security configuration analysis
- Encryption status evaluation
- Access logging and monitoring review
- Compliance with security frameworks (CIS, AWS Security Best Practices)

SECURITY ANALYSIS FOCUS:
- Overly permissive security groups
- Public resource exposure risks
- Weak encryption configurations
- Missing access logging
- IAM policy over-permissions
- Network segmentation issues
- Backup and recovery security
- Data classification and protection

FORBIDDEN OPERATIONS:
- Create or modify IAM policies
- Change security group rules
- Disable security features
- Extract or expose sensitive data
- Modify resource configurations
- Change access permissions
- Disable logging or monitoring

AVAILABLE TOOLS:
- TodoWrite: Track security assessment progress (USE THIS FIRST)
- TodoRead: Check assessment status
- aws_list_resources: Discover resources for security review
- aws_describe_resource: Analyze resource security posture
- aws_find_account: Account lookup (no API calls)
- aws_find_region: Region lookup (no API calls)

CURRENT SECURITY TASK: {task_description}

AWS CONTEXT:
- Account ID: {account_id}
- Region: {region}

EXPECTED OUTPUT:
Provide structured security findings including:
- Security posture summary
- High/Medium/Low risk findings
- Vulnerability identification
- Compliance gap analysis
- Security best practice recommendations
- Prioritized remediation steps
- Risk assessment and impact analysis

SECURITY REPORT STRUCTURE:
1. Executive Security Summary
2. Critical Findings (High Risk)
3. Security Configuration Analysis
4. Compliance Assessment
5. Recommended Security Improvements
6. Implementation Roadmap

SECURITY BEST PRACTICES TO EVALUATE:
- Principle of least privilege
- Defense in depth
- Encryption at rest and in transit
- Network segmentation and isolation
- Access logging and monitoring
- Incident response preparedness
- Backup and disaster recovery
- Data classification and handling

Always use TodoWrite for security assessment planning and track your progress through each security domain."#,
            task_description = task_description,
            account_id = context.account_id,
            region = context.region
        )
    }

    /// Execute a security assessment task with the created agent
    pub async fn execute_security_scan(
        agent: &mut Agent,
        task_description: &str,
    ) -> Result<serde_json::Value, stood::StoodError> {
        info!("🔒 Executing security assessment: {}", task_description);
        
        // Execute the security assessment task
        let result = agent.execute(task_description).await?;
        
        info!("✅ Security assessment completed successfully");
        debug!("Security assessment result: {} chars", result.response.len());

        // Return structured result
        Ok(serde_json::json!({
            "assessment_type": "aws-security-scanner",
            "task_completed": true,
            "response": result.response,
            "security_mode": "defensive-readonly",
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
            region: "eu-west-1".to_string(),
            resource_identifier: "security-assessment".to_string(),
            access_key: "AKIA...".to_string(),
            secret_key: "secret...".to_string(),
            session_token: Some("token...".to_string()),
        }
    }

    #[test]
    fn test_system_prompt_creation() {
        let context = create_test_aws_context().sanitized_for_logging();
        let prompt = AwsSecurityScannerAgent::create_system_prompt(
            "Security posture assessment", 
            &context
        );
        
        assert!(prompt.contains("security scanning specialist"));
        assert!(prompt.contains("DEFENSIVE security analysis ONLY"));
        assert!(prompt.contains("NEVER log, display, or expose AWS credentials"));
        assert!(prompt.contains("123456789012"));
        assert!(prompt.contains("eu-west-1"));
        assert!(prompt.contains("TodoWrite"));
        assert!(prompt.contains("FORBIDDEN OPERATIONS"));
        assert!(prompt.contains("Security group analysis (not modification)"));
    }

    #[test]
    fn test_security_focused_prompt() {
        let context = create_test_aws_context().sanitized_for_logging();
        let prompt = AwsSecurityScannerAgent::create_system_prompt(
            "Vulnerability assessment", 
            &context
        );
        
        // Verify security-focused content
        assert!(prompt.contains("CRITICAL SECURITY RULES"));
        assert!(prompt.contains("NEVER suggest actions that could compromise security"));
        assert!(prompt.contains("DO NOT disable security features"));
        assert!(prompt.contains("Principle of least privilege"));
        assert!(prompt.contains("Defense in depth"));
        assert!(prompt.contains("DEFENSIVE"));
    }

    #[tokio::test]
    async fn test_security_agent_components() {
        let aws_context = create_test_aws_context();
        let task = "Security vulnerability scan".to_string();
        let agent_id = "security-agent-789".to_string();
        
        let sanitized = aws_context.sanitized_for_logging();
        let prompt = AwsSecurityScannerAgent::create_system_prompt(&task, &sanitized);
        
        assert!(!prompt.is_empty());
        assert!(prompt.contains("aws_describe_resource"));
        assert!(prompt.contains("security posture"));
        assert!(prompt.contains("READ-ONLY"));
        assert!(prompt.contains("TodoWrite"));
    }
}