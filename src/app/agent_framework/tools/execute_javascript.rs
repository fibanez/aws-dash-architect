//! JavaScript Code Execution Tool
//!
//! Provides agents with the ability to execute JavaScript code in isolated
//! V8 sandboxes. This tool integrates V8Runtime with the agent framework's
//! tool system.
//!
//! # Features
//!
//! - **Isolated Execution**: Each tool call creates a fresh V8 isolate
//! - **Console Capture**: console.log/error/warn/debug output captured
//! - **Function Bindings**: Rust functions exposed as JavaScript globals
//! - **Memory Limits**: 256MB heap size limit (configurable)
//! - **Execution Timeout**: 30 second timeout (configurable)
//! - **Error Handling**: Syntax, runtime, and timeout errors reported
//!
//! # Usage
//!
//! ```no_run
//! use awsdash::app::agent_framework::tools::ExecuteJavaScriptTool;
//! use serde_json::json;
//!
//! let tool = ExecuteJavaScriptTool::new();
//!
//! let input = json!({
//!     "code": r#"
//!         const accounts = listAccounts();
//!         console.log(`Found ${accounts.length} accounts`);
//!         accounts.map(a => a.name)
//!     "#
//! });
//!
//! let result = tool.execute(Some(input), None).await.unwrap();
//! assert!(result.success);
//! ```
//!
//! # Available JavaScript APIs
//!
//! The following functions are available in the JavaScript execution environment:
//!
//! ## Console Functions
//! - `console.log(...args)` - Log messages to stdout
//! - `console.error(...args)` - Log error messages to stdout
//! - `console.warn(...args)` - Log warning messages to stdout
//! - `console.debug(...args)` - Log debug messages to stdout
//!
//! ## Account Management
//! - `listAccounts()` - List all configured AWS accounts
//!   - Returns: `Array<{ id: string, name: string, alias: string | null, email: string | null }>`
//!
//! # Configuration
//!
//! ```no_run
//! use awsdash::app::agent_framework::tools::ExecuteJavaScriptTool;
//! use awsdash::app::agent_framework::v8_bindings::RuntimeConfig;
//! use std::time::Duration;
//!
//! let config = RuntimeConfig {
//!     timeout: Duration::from_secs(60),  // 60 second timeout
//!     ..Default::default()
//! };
//!
//! let tool = ExecuteJavaScriptTool::with_config(config);
//! ```
//!
//! # Return Values
//!
//! The tool returns a `ToolResult` with:
//! - **Success case**: `success=true`, content contains result and console output
//! - **Error case**: `success=false`, error contains error message and partial console output
//!
//! # Examples
//!
//! ## Simple Calculation
//! ```no_run
//! # use awsdash::app::agent_framework::tools::ExecuteJavaScriptTool;
//! # use serde_json::json;
//! # async {
//! let tool = ExecuteJavaScriptTool::new();
//! let result = tool.execute(Some(json!({ "code": "5 + 3" })), None).await.unwrap();
//! // Result contains: 8
//! # };
//! ```
//!
//! ## Using Bound Functions
//! ```no_run
//! # use awsdash::app::agent_framework::tools::ExecuteJavaScriptTool;
//! # use serde_json::json;
//! # async {
//! let tool = ExecuteJavaScriptTool::new();
//! let result = tool.execute(Some(json!({
//!     "code": r#"
//!         const accounts = listAccounts();
//!         const prod = accounts.find(a => a.alias === 'prod');
//!         prod ? prod.id : null
//!     "#
//! })), None).await.unwrap();
//! // Result contains production account ID
//! # };
//! ```
//!
//! ## Error Handling
//! ```no_run
//! # use awsdash::app::agent_framework::tools::ExecuteJavaScriptTool;
//! # use serde_json::json;
//! # async {
//! let tool = ExecuteJavaScriptTool::new();
//! let result = tool.execute(Some(json!({
//!     "code": "const x = null; x.property"
//! })), None).await.unwrap();
//! // result.success = false
//! // result.error contains TypeError details
//! # };
//! ```

#![warn(clippy::all, rust_2018_idioms)]

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use stood::tools::{Tool, ToolError, ToolResult};
use tracing::{debug, info};

use crate::app::agent_framework::v8_bindings::{ExecutionResult, RuntimeConfig, V8Runtime};

/// JavaScript code execution tool
///
/// Executes JavaScript code in an isolated V8 sandbox with:
/// - Console output capture (console.log/error/warn/debug)
/// - Rust-bound functions (listAccounts, etc.)
/// - Memory limits (256MB default)
/// - Execution timeout (30s default)
///
/// Each execution creates a fresh isolate (no state persistence).
#[derive(Clone, Debug)]
pub struct ExecuteJavaScriptTool {
    /// Runtime configuration (timeout, memory limits, etc.)
    config: RuntimeConfig,
}

impl ExecuteJavaScriptTool {
    /// Create a new JavaScript execution tool with default configuration
    pub fn new() -> Self {
        Self {
            config: RuntimeConfig::default(),
        }
    }

    /// Create a new JavaScript execution tool with custom configuration
    pub fn with_config(config: RuntimeConfig) -> Self {
        Self { config }
    }
}

impl Default for ExecuteJavaScriptTool {
    fn default() -> Self {
        Self::new()
    }
}

/// Tool input format
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExecuteJavaScriptInput {
    /// JavaScript code to execute
    code: String,
}

#[async_trait]
impl Tool for ExecuteJavaScriptTool {
    fn name(&self) -> &str {
        "execute_javascript"
    }

    fn description(&self) -> &str {
        r#"Execute JavaScript code in an isolated V8 sandbox.

This tool allows you to run JavaScript code with access to bound Rust functions
for infrastructure operations. Each execution runs in a fresh isolated environment.

Features:
- Isolated V8 sandbox (256MB memory limit, 30s timeout)
- Console output capture (console.log/error/warn/debug)
- Rust-bound functions available as JavaScript globals
- No Node.js APIs or filesystem access (secure sandbox)

Available JavaScript APIs:
- listAccounts(): List all configured AWS accounts
  Returns: Array<{ id: string, name: string, alias: string|null, email: string|null }>
- listRegions(): List all AWS regions with their codes and names
  Returns: Array<{ code: string, name: string }>
- queryResources(options): Query AWS resources across accounts/regions/types
  Parameters: {
    accounts: string[]|null,     // Account IDs (null = random account)
    regions: string[]|null,      // Region codes (null = us-east-1)
    resourceTypes: string[],     // CloudFormation types (required)
    detail: "count"|"summary"|"tags"|"full"  // Detail level (default: "summary")
  }
  Detail levels:
    - "count": Just the count (fastest, minimal context)
    - "summary": Basic info only - id, name, type, account, region (fast)
    - "tags": Summary + tags array (for tag-based filtering)
    - "full": Complete data including policies/encryption (may wait up to 60s for background loading)
  Returns: {
    status: "success"|"partial"|"error",
    data: Array<resource>,       // Resources at requested detail level (null for count)
    count: number,               // Total count of resources
    detailsLoaded: boolean,      // True if detailed properties are included
    detailsPending: boolean,     // True if background loading is still in progress
    warnings: Array<{ account, region, message }>,
    errors: Array<{ account, region, code, message }>
  }
  **CRITICAL**: Use 'rawProperties' for filtering/sorting by AWS-specific fields!
    - properties: Minimal normalized fields (id, arn, created_date ONLY)
    - rawProperties: Full AWS API response (InstanceType, Runtime, Engine, VpcId, etc.)
    - detailedProperties: Additional Describe API data (policies, encryption settings - for enrichable types)
  Enrichable types (get detailedProperties): Lambda, S3, IAM, SQS, SNS, DynamoDB, KMS, Cognito, etc.
  Examples: AWS::EC2::Instance, AWS::S3::Bucket, AWS::IAM::Role, AWS::Lambda::Function, we support 93 services and 183 resource types
- queryCloudWatchLogEvents(params): Query CloudWatch Logs for analysis and monitoring
  Parameters: { logGroupName: string, accountId: string, region: string, startTime?: number, endTime?: number, filterPattern?: string, limit?: number, logStreamNames?: string[], startFromHead?: boolean }
  Returns: { events: Array<{ timestamp: number, message: string, ingestionTime: number, logStreamName: string }>, nextToken: string|null, totalEvents: number, statistics: { bytesScanned: number, recordsMatched: number, recordsScanned: number } }
  Log Group Patterns: /aws/lambda/{function-name}, /aws/apigateway/{api-name}, /ecs/{cluster-name}, /aws/rds/instance/{instance-id}/error
  Filter Patterns: "ERROR" (simple text), '{ $.level = "ERROR" }' (JSON), "[timestamp, request_id, level, msg]" (structured)
  Time format: Unix milliseconds (use Date.now() - 3600000 for last hour)
- getCloudTrailEvents(params): Query CloudTrail events for governance, compliance, and security analysis
  Parameters: { accountId: string, region: string, startTime?: number, endTime?: number, lookupAttributes?: Array<{attributeKey: string, attributeValue: string}>, maxResults?: number }
  Returns: { events: Array<{ eventId: string, eventName: string, eventTime: number, eventSource: string, username: string, resources: Array<{resourceType: string, resourceName: string}>, errorCode?: string }>, nextToken: string|null, totalEvents: number }
  Lookup Attribute Keys: "EventId", "EventName", "ResourceType", "ResourceName", "Username", "EventSource", "AccessKeyId", "ReadOnly"
  **CRITICAL** ResourceName Format: Use resource NAME/ID, NOT ARN - Lambda: "my-function" (NOT arn:aws:lambda:...), EC2: "i-1234567890abcdef0" (NOT arn:aws:ec2:...)
  **CRITICAL** API Limitation: ONLY ONE lookupAttribute allowed per query (AWS restriction) - if multiple provided, only LAST one is used
  Common Event Names: RunInstances, TerminateInstances, CreateBucket, DeleteBucket, PutBucketPolicy, CreateFunction, UpdateFunctionCode
  Default Behavior: Automatically fetches at least 100 events (2 pages) for better coverage
  CloudTrail Delay: Events appear 5-15 minutes after the API call

**PROPERTY ACCESS GUIDELINES** (queryResources):
The 'properties' field is MINIMAL (only id/arn/created_date). Use 'rawProperties' for AWS-specific fields:

1. EC2 Instances (AWS::EC2::Instance) - rawProperties fields:
   InstanceType, State, VpcId, SubnetId, PrivateIpAddress, PublicIpAddress, LaunchTime
   Example: resources.filter(r => r.rawProperties.InstanceType === 't3.micro')

2. Lambda Functions (AWS::Lambda::Function) - rawProperties fields:
   FunctionName, Runtime, Handler, MemorySize, Timeout, State, LastModified
   Example: resources.filter(r => r.rawProperties.Runtime === 'python3.11')

3. S3 Buckets (AWS::S3::Bucket) - rawProperties fields:
   Name, CreationDate
   Example: resources.filter(r => r.rawProperties.Name.includes('prod'))

4. RDS Instances (AWS::RDS::DBInstance) - rawProperties fields:
   DBInstanceIdentifier, DBInstanceClass, Engine, EngineVersion, DBInstanceStatus
   Example: resources.filter(r => r.rawProperties.Engine === 'postgres')

5. IAM Roles (AWS::IAM::Role) - rawProperties fields:
   RoleName, Path, Arn, CreateDate, MaxSessionDuration
   Example: resources.filter(r => r.rawProperties.Path === '/service-role/')

**Common Patterns**:
❌ WRONG: resources.filter(r => r.properties.InstanceType === 't3.micro')
✅ CORRECT: resources.filter(r => r.rawProperties.InstanceType === 't3.micro')
✅ Use 'status' field: resources.filter(r => r.status === 'running')
✅ Use 'tags' array: resources.filter(r => r.tags.some(t => t.key === 'Environment'))

Input Parameters:
- code: JavaScript code string to execute

Output:
- success: Whether execution succeeded
- result: Return value from the JavaScript code (JSON)
- stdout: Console output (console.log/warn/debug)
- stderr: Error messages (console.error + exceptions)
- execution_time_ms: Time taken to execute

Examples:
1. List all accounts:
   {"code": "const accounts = listAccounts(); accounts;"}

2. List all AWS regions:
   {"code": "const regions = listRegions(); regions;"}

3. Filter regions by prefix:
   {"code": "const regions = listRegions(); regions.filter(r => r.code.startsWith('us-'));"}

4. Filter accounts:
   {"code": "const accounts = listAccounts(); accounts.filter(a => a.alias === 'prod');"}

5. Process data with console output:
   {"code": "const regions = listRegions(); console.log(`Found ${regions.length} regions`); regions.map(r => r.name);"}

6. Query EC2 instances:
   {"code": "const instances = queryResources({ accounts: null, regions: null, resourceTypes: ['AWS::EC2::Instance'] }); instances;"}

7. Find t3.micro instances:
   {"code": "const instances = queryResources({ accounts: null, regions: ['us-east-1'], resourceTypes: ['AWS::EC2::Instance'] }); instances.filter(i => i.rawProperties.InstanceType === 't3.micro');"}

8. Query multiple resource types:
   {"code": "const resources = queryResources({ accounts: listAccounts().map(a => a.id), regions: ['us-east-1'], resourceTypes: ['AWS::EC2::Instance', 'AWS::S3::Bucket'] }); resources;"}

9. Query CloudWatch Logs for Lambda errors:
   {"code": "const errors = queryCloudWatchLogEvents({ logGroupName: '/aws/lambda/my-function', accountId: '123456789012', region: 'us-east-1', filterPattern: 'ERROR', startTime: Date.now() - (60 * 60 * 1000), limit: 100 }); errors.events;"}

10. Find recent API Gateway 4xx/5xx errors:
   {"code": "const apiLogs = queryCloudWatchLogEvents({ logGroupName: '/aws/apigateway/my-api', accountId: '123456789012', region: 'us-east-1', filterPattern: '[ip, timestamp, method, path, status>=400]', limit: 500 }); apiLogs.events.map(e => e.message);"}

11. Query CloudTrail for EC2 instance changes:
   {"code": "const events = getCloudTrailEvents({ accountId: '123456789012', region: 'us-east-1', lookupAttributes: [{ attributeKey: 'ResourceType', attributeValue: 'AWS::EC2::Instance' }] }); events.events.map(e => ({ time: new Date(e.eventTime).toISOString(), action: e.eventName, user: e.username }));"}

12. Find CloudTrail security events (failed API calls):
   {"code": "const events = getCloudTrailEvents({ accountId: '123456789012', region: 'us-east-1', startTime: Date.now() - (24 * 60 * 60 * 1000) }); events.events.filter(e => e.errorCode).map(e => ({ event: e.eventName, error: e.errorCode, user: e.username }));"}

13. Find Lambda functions by runtime:
   {"code": "const fns = queryResources({ accounts: null, regions: null, resourceTypes: ['AWS::Lambda::Function'] }); fns.filter(f => f.rawProperties.Runtime === 'python3.11');"}

14. Find RDS PostgreSQL databases:
   {"code": "const dbs = queryResources({ accounts: null, regions: null, resourceTypes: ['AWS::RDS::DBInstance'] }); dbs.filter(d => d.rawProperties.Engine === 'postgres');"}

15. Sort instances by launch time:
   {"code": "const instances = queryResources({ accounts: null, regions: null, resourceTypes: ['AWS::EC2::Instance'] }); instances.sort((a, b) => new Date(a.rawProperties.LaunchTime) - new Date(b.rawProperties.LaunchTime));"}

16. Filter S3 buckets by creation date:
   {"code": "const buckets = queryResources({ accounts: null, regions: null, resourceTypes: ['AWS::S3::Bucket'] }); buckets.filter(b => new Date(b.rawProperties.CreationDate) > new Date('2024-01-01'));"}

Return Values:
- Use the last expression as the return value (no 'return' statement needed)
- Return values are automatically converted to JSON
- Complex objects, arrays, and primitives all supported
- Do NOT use 'return' statements - they cause syntax errors at script level"#
    }

    fn parameters_schema(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "code": {
                    "type": "string",
                    "description": "JavaScript code to execute. Use last expression as return value (no 'return' statement needed).",
                    "examples": [
                        "const accounts = listAccounts(); accounts;",
                        "console.log('Hello'); 42;",
                        "const accounts = listAccounts(); accounts.filter(a => a.alias === 'prod');"
                    ]
                }
            },
            "required": ["code"]
        })
    }

    async fn execute(
        &self,
        parameters: Option<Value>,
        _agent_context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        let start_time = std::time::Instant::now();
        info!(
            "🚀 execute_javascript executing with parameters: {:?}",
            parameters
        );

        // Parse parameters
        let params = parameters.ok_or_else(|| ToolError::InvalidParameters {
            message: "Missing parameters for execute_javascript".to_string(),
        })?;

        let input: ExecuteJavaScriptInput =
            serde_json::from_value(params.clone()).map_err(|e| ToolError::InvalidParameters {
                message: format!("Invalid input format: {}", e),
            })?;

        // Validate code is not empty
        if input.code.trim().is_empty() {
            return Ok(ToolResult::error(
                "Code parameter cannot be empty".to_string(),
            ));
        }

        debug!("Executing JavaScript code: {} characters", input.code.len());

        // Log the JavaScript code being executed
        info!("📝 JavaScript code being executed:\n{}", input.code);

        // Log to per-agent log if available
        if let Some(logger) = crate::app::agent_framework::agent_logger::get_current_agent_logger()
        {
            if let Some(agent_type) = crate::app::agent_framework::get_current_agent_type() {
                logger.log_tool_start(&agent_type, "execute_javascript", &params);
            }
        }

        // Execute JavaScript with error handling
        let tool_result = match execute_with_error_handling(&input.code, &self.config) {
            Ok(result) => {
                // Log successful execution with result details
                if result.success {
                    let result_preview = if let Some(content) = result.content.as_object() {
                        if let Some(result_value) = content.get("result") {
                            serde_json::to_string(result_value)
                                .unwrap_or_else(|_| "null".to_string())
                                .to_string()
                        } else {
                            "undefined".to_string()
                        }
                    } else {
                        "undefined".to_string()
                    };

                    info!(
                        "✅ JavaScript execution succeeded - Result: {}",
                        result_preview
                    );

                    // Log console output if present
                    if let Some(content) = result.content.as_object() {
                        if let Some(stdout) = content.get("stdout").and_then(|v| v.as_str()) {
                            if !stdout.is_empty() {
                                info!("📺 Console output:\n{}", stdout);
                            }
                        }
                    }
                } else {
                    // Log error details
                    if let Some(error_msg) = result.error.as_ref() {
                        info!("❌ JavaScript execution failed - Error: {}", error_msg);
                    } else {
                        info!("❌ JavaScript execution failed - Error: Unknown error");
                    }
                }
                result
            }
            Err(e) => {
                // V8 initialization or catastrophic failure
                info!("💥 JavaScript execution catastrophic failure: {}", e);
                ToolResult::error(format!(
                    "JavaScript execution failed: {}\n\n\
                     This is likely an internal error with the V8 runtime.",
                    e
                ))
            }
        };

        let elapsed = start_time.elapsed();
        info!("⏱️ execute_javascript total duration: {:?}", elapsed);

        // Log completion/failure to per-agent log if available
        if let Some(logger) = crate::app::agent_framework::agent_logger::get_current_agent_logger()
        {
            if let Some(agent_type) = crate::app::agent_framework::get_current_agent_type() {
                if tool_result.success {
                    logger.log_tool_complete(
                        &agent_type,
                        "execute_javascript",
                        Some(&tool_result.content),
                        elapsed,
                    );
                } else if let Some(error_msg) = tool_result.error.as_ref() {
                    logger.log_tool_failed(&agent_type, "execute_javascript", error_msg, elapsed);
                } else {
                    logger.log_tool_failed(
                        &agent_type,
                        "execute_javascript",
                        "Unknown error",
                        elapsed,
                    );
                }
            }
        }

        Ok(tool_result)
    }
}

/// Execute JavaScript with comprehensive error handling
fn execute_with_error_handling(code: &str, config: &RuntimeConfig) -> anyhow::Result<ToolResult> {
    // Create V8 runtime with configuration
    // Note: V8Runtime automatically registers console and function bindings
    let runtime = V8Runtime::with_config(config.clone());

    // Execute JavaScript code
    let execution_result = runtime
        .execute(code)
        .map_err(|e| anyhow::anyhow!("Failed to execute JavaScript: {}", e))?;

    // Convert ExecutionResult to ToolResult
    Ok(format_execution_result(execution_result))
}

/// Format ExecutionResult as ToolResult for agent consumption
fn format_execution_result(result: ExecutionResult) -> ToolResult {
    if result.success {
        // Format successful execution
        let stdout_display = if result.stdout.is_empty() {
            "(no output)".to_string()
        } else {
            result.stdout.clone()
        };

        let result_display = result
            .result
            .as_ref()
            .and_then(|v| serde_json::from_str::<serde_json::Value>(v).ok())
            .map(|v| serde_json::to_string_pretty(&v).unwrap_or_else(|_| "null".to_string()))
            .unwrap_or_else(|| "undefined".to_string());

        let output = format!(
            "Execution completed successfully in {}ms\n\n\
             === Result ===\n\
             {}\n\n\
             === Console Output ===\n\
             {}",
            result.execution_time_ms, result_display, stdout_display
        );

        // Parse result as JSON for structured output
        let result_json = result
            .result
            .as_ref()
            .and_then(|v| serde_json::from_str::<serde_json::Value>(v).ok())
            .unwrap_or(serde_json::Value::Null);

        ToolResult::success(serde_json::json!({
            "success": true,
            "result": result_json,
            "stdout": result.stdout,
            "stderr": result.stderr,
            "execution_time_ms": result.execution_time_ms,
            "output": output
        }))
    } else {
        // Format error execution
        let stdout_display = if result.stdout.is_empty() {
            "(no output)".to_string()
        } else {
            result.stdout.clone()
        };

        let error_msg = format!(
            "Execution failed after {}ms\n\n\
             === Error ===\n\
             {}\n\n\
             === Console Output (before error) ===\n\
             {}",
            result.execution_time_ms, result.stderr, stdout_display
        );

        ToolResult::error(error_msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::agent_framework::v8_bindings::bindings::accounts::set_global_aws_identity;
    use crate::app::aws_identity::{AwsAccount, AwsIdentityCenter, LoginState};
    use std::sync::{Arc, Mutex};

    /// Setup test identity for JavaScript tests that use listAccounts()
    fn setup_test_identity() {
        let mut identity_center = AwsIdentityCenter::new(
            "https://test.awsapps.com/start".to_string(),
            "test-role".to_string(),
            "us-east-1".to_string(),
        );

        // Add test accounts (2 accounts to match bindings/accounts.rs tests)
        identity_center.accounts = vec![
            AwsAccount {
                account_id: "123456789012".to_string(),
                account_name: "Test Production Account".to_string(),
                account_email: Some("prod@test.com".to_string()),
                role_name: "test-role".to_string(),
                credentials: None,
            },
            AwsAccount {
                account_id: "987654321098".to_string(),
                account_name: "Test Development Account".to_string(),
                account_email: Some("dev@test.com".to_string()),
                role_name: "test-role".to_string(),
                credentials: None,
            },
        ];
        identity_center.login_state = LoginState::LoggedIn;

        // Set globally for tests
        set_global_aws_identity(Some(Arc::new(Mutex::new(identity_center))));
    }

    #[tokio::test]
    async fn test_tool_metadata() {
        let tool = ExecuteJavaScriptTool::new();

        assert_eq!(tool.name(), "execute_javascript");
        assert!(!tool.description().is_empty());

        let schema = tool.parameters_schema();
        assert_eq!(schema["type"], "object");
        assert!(schema["properties"]["code"].is_object());
        assert_eq!(schema["required"][0], "code");
    }

    #[tokio::test]
    async fn test_empty_code_validation() {
        let tool = ExecuteJavaScriptTool::new();

        let params = serde_json::json!({ "code": "" });
        let result = tool.execute(Some(params), None).await.unwrap();

        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("empty"));
    }

    #[tokio::test]
    async fn test_whitespace_only_code_validation() {
        let tool = ExecuteJavaScriptTool::new();

        let params = serde_json::json!({ "code": "   \n\t  " });
        let result = tool.execute(Some(params), None).await.unwrap();

        assert!(!result.success);
        assert!(result.error.as_ref().unwrap().contains("empty"));
    }

    #[tokio::test]
    async fn test_invalid_input_format() {
        let tool = ExecuteJavaScriptTool::new();

        let params = serde_json::json!({ "invalid_field": "value" });
        let result = tool.execute(Some(params), None).await;

        assert!(result.is_err());
        if let Err(ToolError::InvalidParameters { message }) = result {
            assert!(message.contains("Invalid input"));
        } else {
            panic!("Expected InvalidParameters error");
        }
    }

    // V8Runtime integration tests

    #[tokio::test]
    async fn test_basic_javascript_execution() {
        use crate::app::agent_framework::v8_bindings::initialize_v8_platform;
        let _ = initialize_v8_platform();

        let tool = ExecuteJavaScriptTool::new();

        let params = serde_json::json!({
            "code": "const x = 5 + 3; x"
        });

        let result = tool.execute(Some(params), None).await.unwrap();

        assert!(result.success, "Execution failed: {:?}", result.error);
        let content_obj = result.content.as_object().unwrap();
        assert_eq!(content_obj["success"], true);
        assert_eq!(content_obj["result"], 8);
    }

    #[tokio::test]
    async fn test_console_output_capture() {
        use crate::app::agent_framework::v8_bindings::initialize_v8_platform;
        let _ = initialize_v8_platform();

        let tool = ExecuteJavaScriptTool::new();

        let params = serde_json::json!({
            "code": "console.log('Hello'); console.log('World'); 42"
        });

        let result = tool.execute(Some(params), None).await.unwrap();

        assert!(result.success);
        let content_obj = result.content.as_object().unwrap();
        let stdout = content_obj["stdout"].as_str().unwrap();
        assert!(stdout.contains("Hello"));
        assert!(stdout.contains("World"));
        assert_eq!(content_obj["result"], 42);
    }

    #[tokio::test]
    async fn test_syntax_error_handling() {
        use crate::app::agent_framework::v8_bindings::initialize_v8_platform;
        let _ = initialize_v8_platform();

        let tool = ExecuteJavaScriptTool::new();

        let params = serde_json::json!({
            "code": "const x = ;"  // Invalid syntax
        });

        let result = tool.execute(Some(params), None).await.unwrap();

        assert!(!result.success);
        let error_msg = result.error.unwrap();
        assert!(error_msg.contains("Error") || error_msg.contains("SyntaxError"));
    }

    #[tokio::test]
    async fn test_runtime_error_handling() {
        use crate::app::agent_framework::v8_bindings::initialize_v8_platform;
        let _ = initialize_v8_platform();

        let tool = ExecuteJavaScriptTool::new();

        let params = serde_json::json!({
            "code": "const x = null; x.property"  // null reference error
        });

        let result = tool.execute(Some(params), None).await.unwrap();

        assert!(!result.success);
        let error_msg = result.error.unwrap();
        assert!(error_msg.contains("Error") || error_msg.contains("TypeError"));
    }

    #[tokio::test]
    async fn test_function_binding_available() {
        use crate::app::agent_framework::v8_bindings::initialize_v8_platform;
        let _ = initialize_v8_platform();
        setup_test_identity();

        let tool = ExecuteJavaScriptTool::new();

        let params = serde_json::json!({
            "code": "const accounts = listAccounts(); accounts.length"
        });

        let result = tool.execute(Some(params), None).await.unwrap();

        assert!(result.success);
        let content_obj = result.content.as_object().unwrap();
        // Should return the number of accounts from test data (2 accounts)
        assert_eq!(content_obj["result"], 2);
    }

    #[tokio::test]
    async fn test_complex_javascript_execution() {
        use crate::app::agent_framework::v8_bindings::initialize_v8_platform;
        let _ = initialize_v8_platform();
        setup_test_identity();

        let tool = ExecuteJavaScriptTool::new();

        let params = serde_json::json!({
            "code": r#"
                const accounts = listAccounts();
                console.log(`Found ${accounts.length} accounts`);

                const prodAccounts = accounts.filter(a => a.alias === 'prod');
                console.log(`Production accounts: ${prodAccounts.length}`);

                // Return the object directly - V8Runtime will JSON.stringify it
                const result = {
                    total: accounts.length,
                    prod: prodAccounts.length,
                    names: accounts.map(a => a.name)
                };
                result;
            "#
        });

        let result = tool.execute(Some(params), None).await.unwrap();

        assert!(result.success, "Execution failed: {:?}", result.error);
        let content_obj = result.content.as_object().unwrap();

        let stdout = content_obj["stdout"].as_str().unwrap();
        assert!(stdout.contains("Found 2 accounts"));
        assert!(stdout.contains("Production accounts: 0")); // No 'prod' alias in test data

        let result_value = &content_obj["result"];
        assert_eq!(result_value["total"], 2);
        assert_eq!(result_value["prod"], 0);
        assert!(result_value["names"].is_array());
    }

    // Tool trait implementation tests

    #[test]
    fn test_tool_implements_tool_trait() {
        // Compile-time verification that ExecuteJavaScriptTool implements Tool
        fn assert_tool<T: stood::tools::Tool>(_tool: &T) {}

        let tool = ExecuteJavaScriptTool::new();
        assert_tool(&tool);
    }

    #[test]
    fn test_custom_config() {
        use std::time::Duration;

        let config = RuntimeConfig {
            timeout: Duration::from_secs(60),
            ..Default::default()
        };

        let tool = ExecuteJavaScriptTool::with_config(config);
        assert_eq!(tool.name(), "execute_javascript");
    }
}
