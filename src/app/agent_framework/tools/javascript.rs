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
    /// Optional intent describing what the agent is trying to accomplish
    /// This is logged for debugging but doesn't affect execution
    #[serde(skip_serializing_if = "Option::is_none")]
    intent: Option<String>,
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
- loadCache(options): Load AWS resources into cache, returns counts only (~99% context reduction)
  Parameters: {
    accounts: string[]|null,     // Account IDs (null = common regions)
    regions: string[]|null,      // Region codes (null = us-east-1, us-west-2, eu-west-1, ap-southeast-1)
    resourceTypes: string[]      // CloudFormation types (REQUIRED)
  }
  Returns: {
    status: "success"|"partial"|"error",
    countByScope: { "account:region:type": count },  // e.g., "123:us-east-1:AWS::EC2::Instance": 45
    totalCount: number,
    warnings: Array<{ account, region, message }>,
    errors: Array<{ account, region, code, message }>,
    accountsQueried: string[],
    regionsQueried: string[],
    loadTimestampUtc: string
  }
  **KEY**: Returns COUNTS not resources - minimizes context usage. Resources stay in V8 cache.
- getResourceSchema(resourceType): Get ONE example resource to see available properties
  Parameters: resourceType (string) - e.g., "AWS::EC2::Instance"
  Returns: {
    status: "success"|"not_found",
    resourceType: string,
    exampleResource: { resourceId, displayName, accountId, region, properties, tags, status },
    cacheStats: { totalCount, accountCount, regionCount },
    message: string (if not_found)
  }
  **CRITICAL: Use getResourceSchema() FIRST to understand resource structure before filtering**
- queryCachedResources(options): Query actual resources from cache for filtering/analysis
  Parameters: {
    accounts: string[]|null,     // Account IDs to filter (null = all cached accounts)
    regions: string[]|null,      // Region codes to filter (null = all cached regions)
    resourceTypes: string[]      // Resource types to query (REQUIRED, can be multiple)
  }
  Returns: {
    status: "success"|"not_found",
    resources: Array<ResourceEntry>,  // Full resource objects with properties, tags, etc.
    count: number,
    accountsWithData: string[],
    regionsWithData: string[],
    resourceTypesFound: string[],
    message: string (if not_found: "Call loadCache() first...")
  }
  **WORKFLOW**: (1) loadCache() to populate, (2) getResourceSchema() to see structure, (3) queryCachedResources() to filter
  **CRITICAL**: ALWAYS call getResourceSchema() FIRST to understand property names before filtering!
  **CRITICAL**: Properties are MERGED - properties, raw_properties, detailed_properties are combined into single "properties" object
- showInExplorer(config): Open Explorer window with dynamic configuration
  Parameters: {
    accounts: string[],
    regions: string[],
    resourceTypes: string[],
    grouping: { type: "ByAccount"|"ByRegion"|"ByResourceType"|"ByTag", key?: string },
    tagFilters: { operator: "And"|"Or", filters: [...] },
    searchFilter: string,
    title: string
  }
  Returns: { status: "success"|"error", message: string, resourcesDisplayed: number }
  **Opens UI window - use for visualization**
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

**RESOURCE QUERY WORKFLOW** (Context-Optimized - TWO EXECUTION PATTERN):
The workflow minimizes LLM context by separating data loading from analysis.
CRITICAL: This is a TWO-STEP process requiring TWO separate JavaScript executions.

**EXECUTION 1: Load Cache + Get Schema** (Returns metadata to LLM)
Execute this FIRST to understand what data exists and what properties are available:

  (async () => {
    // Load resources into cache (returns counts only, not full data)
    const loadResult = await loadCache({
      accounts: listAccounts().map(a => a.id),
      regions: ['us-east-1', 'us-west-2'],
      resourceTypes: ['AWS::EC2::SecurityGroup']
    });
    console.log('Loaded:', JSON.stringify(loadResult, null, 2));
    // Returns: { countByScope: {...}, totalCount: 234 }

    // Get schema to discover available properties
    const schema = await getResourceSchema('AWS::EC2::SecurityGroup');
    console.log('Available properties:', Object.keys(schema.exampleResource.properties));
    console.log('Example resource:', JSON.stringify(schema.exampleResource, null, 2));
    // Returns: { exampleResource: { properties: {...}, tags: [...] } }
    // Properties are MERGED - all fields from Phase 1 and Phase 2 are in "properties"

    // Return both results so LLM can see counts and schema structure
    return {
      loaded: loadResult,
      schema: schema.exampleResource
    };
  })()

**After Execution 1**: You (the LLM) will see:
- How many resources were loaded (totalCount)
- What properties are available (from schema.properties)
- Example values for each property
- Nested structure (e.g., IpPermissions array with FromPort, ToPort, IpRanges fields)

**EXECUTION 2: Query + Filter + Process** (Write this AFTER seeing Execution 1 results)
NOW you know the schema structure, so write JavaScript to query and filter:

  (async () => {
    // Query cached resources (already loaded in Execution 1)
    const sgs = await queryCachedResources({
      accounts: null,  // All cached accounts
      regions: null,   // All cached regions
      resourceTypes: ['AWS::EC2::SecurityGroup']
    });
    console.log(`Querying ${sgs.count} security groups`);

    // Filter using properties discovered from schema
    // You know from Execution 1 that sg.properties.IpPermissions exists
    const openSSH = sgs.resources.filter(sg => {
      const rules = sg.properties.IpPermissions || [];
      return rules.some(rule => {
        const fromPort = rule.FromPort || 0;
        const toPort = rule.ToPort || 65535;
        const hasPort22 = fromPort <= 22 && 22 <= toPort;
        const openToWorld = (rule.IpRanges || []).some(r => r.CidrIp === '0.0.0.0/0') ||
                            (rule.Ipv6Ranges || []).some(r => r.CidrIpv6 === '::/0');
        return hasPort22 && openToWorld;
      });
    });
    console.log(`Found ${openSSH.length} security groups with SSH open to world`);

    // Return filtered/aggregated results (NOT raw arrays)
    // If > 10 results, use showInExplorer() instead
    if (openSSH.length > 10) {
      showInExplorer({
        title: 'Security Groups with Public SSH',
        resources: openSSH,
        accounts: [...new Set(openSSH.map(r => r.accountId))],
        regions: [...new Set(openSSH.map(r => r.region))],
        resourceTypes: ['AWS::EC2::SecurityGroup']
      });
      return { count: openSSH.length, message: 'Results in Explorer window' };
    }

    // For <= 10 results, return brief summary
    return openSSH.map(sg => ({
      id: sg.resourceId,
      name: sg.displayName,
      account: sg.accountId,
      region: sg.region,
      vpcId: sg.properties.VpcId
    }));
  })()

**WHY TWO EXECUTIONS?**
- Execution 1: You discover what properties exist (e.g., "IpPermissions", "FromPort")
- Execution 2: You write correct filter logic using those exact property names
- Trying to do both in one execution means guessing at property structure = errors

Input Parameters:
- code: JavaScript code string to execute

Output:
- success: Whether execution succeeded
- result: Return value from the JavaScript code (JSON)
- stdout: Console output (console.log/warn/debug)
- stderr: Error messages (console.error + exceptions)
- execution_time_ms: Time taken to execute

**IMPORTANT - Logging Objects**:
When logging objects with console.log(), they display as '[object Object]'.
Use JSON.stringify() to see actual content:
  ❌ console.log('Result:', result);           // Shows: Result: [object Object]
  ✅ console.log('Result:', JSON.stringify(result, null, 2));  // Shows actual JSON

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

5b. Log object with JSON.stringify:
   {"code": "const result = await loadCache({ accounts: null, regions: null, resourceTypes: ['AWS::EC2::Instance'] }); console.log('Load result:', JSON.stringify(result, null, 2)); result;"}

6. Load EC2 instances into cache (get counts):
   {"code": "const result = await loadCache({ accounts: null, regions: null, resourceTypes: ['AWS::EC2::Instance'] }); result;"}

7. Discover EC2 instance schema:
   {"code": "const schema = await getResourceSchema('AWS::EC2::Instance'); schema.exampleResource;"}

8. Load multiple resource types:
   {"code": "const result = await loadCache({ accounts: listAccounts().map(a => a.id), regions: ['us-east-1'], resourceTypes: ['AWS::EC2::Instance', 'AWS::S3::Bucket'] }); result.countByScope;"}

8b. Query cached resources and filter (CORRECT - uses merged properties):
   {"code": "await loadCache({ accounts: null, regions: null, resourceTypes: ['AWS::EC2::Instance'] }); const schema = await getResourceSchema('AWS::EC2::Instance'); console.log('Properties:', Object.keys(schema.exampleResource.properties)); const result = await queryCachedResources({ accounts: null, regions: null, resourceTypes: ['AWS::EC2::Instance'] }); result.resources.filter(i => i.properties.InstanceType === 't3.micro');"}

8c. Complete workflow - find security groups with port 22 open to world:
   {"code": "await loadCache({ accounts: null, regions: null, resourceTypes: ['AWS::EC2::SecurityGroup'] }); const schema = await getResourceSchema('AWS::EC2::SecurityGroup'); console.log('Available properties:', Object.keys(schema.exampleResource.properties)); const sgs = await queryCachedResources({ accounts: null, regions: null, resourceTypes: ['AWS::EC2::SecurityGroup'] }); const openSSH = sgs.resources.filter(sg => { const rules = sg.properties.IpPermissions || []; return rules.some(rule => { const fromPort = rule.FromPort || 0; const toPort = rule.ToPort || 65535; const hasPort22 = fromPort <= 22 && 22 <= toPort; const openIPv4 = (rule.IpRanges || []).some(r => r.CidrIp === '0.0.0.0/0'); const openIPv6 = (rule.Ipv6Ranges || []).some(r => r.CidrIpv6 === '::/0'); return hasPort22 && (openIPv4 || openIPv6); }); }); openSSH.map(sg => ({ id: sg.resourceId, name: sg.displayName, account: sg.accountId, region: sg.region, vpcId: sg.properties.VpcId }));"}

8d. Aggregate instance counts by type and region:
   {"code": "await loadCache({ accounts: null, regions: null, resourceTypes: ['AWS::EC2::Instance'] }); const instances = await queryCachedResources({ accounts: null, regions: null, resourceTypes: ['AWS::EC2::Instance'] }); const byTypeAndRegion = instances.resources.reduce((acc, i) => { const key = `${i.region}:${i.properties.InstanceType}`; acc[key] = (acc[key] || 0) + 1; return acc; }, {}); byTypeAndRegion;"}

8e. Find S3 buckets without encryption:
   {"code": "await loadCache({ accounts: null, regions: null, resourceTypes: ['AWS::S3::Bucket'] }); const schema = await getResourceSchema('AWS::S3::Bucket'); const buckets = await queryCachedResources({ accounts: null, regions: null, resourceTypes: ['AWS::S3::Bucket'] }); const unencrypted = buckets.resources.filter(b => !b.properties.BucketEncryption); unencrypted.map(b => ({ name: b.resourceId, account: b.accountId }));"}

8f. Find IAM roles with specific trust relationship:
   {"code": "await loadCache({ accounts: null, regions: null, resourceTypes: ['AWS::IAM::Role'] }); const roles = await queryCachedResources({ accounts: null, regions: null, resourceTypes: ['AWS::IAM::Role'] }); const ec2Roles = roles.resources.filter(r => { const policy = r.properties.AssumeRolePolicyDocument; return policy && JSON.stringify(policy).includes('ec2.amazonaws.com'); }); ec2Roles.map(r => ({ name: r.resourceId, account: r.accountId }));"}

9. Query CloudWatch Logs for Lambda errors:
   {"code": "const errors = queryCloudWatchLogEvents({ logGroupName: '/aws/lambda/my-function', accountId: '123456789012', region: 'us-east-1', filterPattern: 'ERROR', startTime: Date.now() - (60 * 60 * 1000), limit: 100 }); errors.events;"}

10. Find recent API Gateway 4xx/5xx errors:
   {"code": "const apiLogs = queryCloudWatchLogEvents({ logGroupName: '/aws/apigateway/my-api', accountId: '123456789012', region: 'us-east-1', filterPattern: '[ip, timestamp, method, path, status>=400]', limit: 500 }); apiLogs.events.map(e => e.message);"}

11. Query CloudTrail for EC2 instance changes:
   {"code": "const events = getCloudTrailEvents({ accountId: '123456789012', region: 'us-east-1', lookupAttributes: [{ attributeKey: 'ResourceType', attributeValue: 'AWS::EC2::Instance' }] }); events.events.map(e => ({ time: new Date(e.eventTime).toISOString(), action: e.eventName, user: e.username }));"}

12. Find CloudTrail security events (failed API calls):
   {"code": "const events = getCloudTrailEvents({ accountId: '123456789012', region: 'us-east-1', startTime: Date.now() - (24 * 60 * 60 * 1000) }); events.events.filter(e => e.errorCode).map(e => ({ event: e.eventName, error: e.errorCode, user: e.username }));"}

13. Load Lambda functions and get schema:
   {"code": "await loadCache({ accounts: null, regions: null, resourceTypes: ['AWS::Lambda::Function'] }); const schema = await getResourceSchema('AWS::Lambda::Function'); schema.exampleResource.properties;"}

14. Load RDS databases and check count:
   {"code": "const result = await loadCache({ accounts: null, regions: null, resourceTypes: ['AWS::RDS::DBInstance'] }); result.totalCount;"}

15. Load instances across all accounts:
   {"code": "const result = await loadCache({ accounts: listAccounts().map(a => a.id), regions: ['us-east-1', 'us-west-2'], resourceTypes: ['AWS::EC2::Instance'] }); result;"}

16. Visualize S3 buckets in Explorer:
   {"code": "await loadCache({ accounts: null, regions: null, resourceTypes: ['AWS::S3::Bucket'] }); showInExplorer({ resourceTypes: ['AWS::S3::Bucket'], grouping: { type: 'ByAccount' }, title: 'S3 Buckets by Account' });"}

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
                    "description": "JavaScript code to execute. For async operations, use IIFE pattern: (async () => { /* code */ return result; })(). CRITICAL: Include explicit 'return' statement in IIFE to avoid returning empty object {}.",
                    "examples": [
                        // Basic queries
                        "const accounts = listAccounts(); accounts;",
                        "const regions = listRegions(); regions.filter(r => r.code.startsWith('us-'));",

                        // EXECUTION 1: Load cache + get schema (discover what properties exist)
                        "(async () => { const loadResult = await loadCache({ accounts: null, regions: null, resourceTypes: ['AWS::EC2::SecurityGroup'] }); const schema = await getResourceSchema('AWS::EC2::SecurityGroup'); return { loaded: loadResult, schema: schema.exampleResource }; })()",

                        // EXECUTION 2: Query + filter (write THIS after seeing Execution 1 results)
                        "(async () => { const sgs = await queryCachedResources({ resource_types: ['AWS::EC2::SecurityGroup'] }); const vulnerable = sgs.resources.filter(sg => { const rules = sg.properties.IpPermissions || []; return rules.some(rule => rule.FromPort === 22 && (rule.IpRanges || []).some(r => r.CidrIp === '0.0.0.0/0')); }); if (vulnerable.length > 10) { showInExplorer({ title: 'Vulnerable SGs', resources: vulnerable, accounts: [...new Set(vulnerable.map(r => r.accountId))], regions: [...new Set(vulnerable.map(r => r.region))], resourceTypes: ['AWS::EC2::SecurityGroup'] }); return { count: vulnerable.length, message: 'Results in Explorer' }; } return vulnerable.map(sg => ({ id: sg.properties.GroupId, name: sg.properties.GroupName, account: sg.accountId, region: sg.region })); })()",

                        // Context-efficient aggregation: Return counts, not full arrays
                        "(async () => { const buckets = await queryCachedResources({ resource_types: ['AWS::S3::Bucket'] }); const byEncryption = buckets.resources.reduce((acc, b) => { const enc = b.properties.BucketEncryption?.Rules?.[0]?.ApplyServerSideEncryptionByDefault?.SSEAlgorithm || 'NONE'; acc[enc] = (acc[enc] || 0) + 1; return acc; }, {}); return byEncryption; })()"
                    ]
                },
                "intent": {
                    "type": "string",
                    "description": "Optional: Describe what you're trying to accomplish with this code. This helps with debugging and understanding the agent's reasoning.",
                    "examples": [
                        "Exploring available AWS accounts to understand environment structure",
                        "Checking if S3 buckets have encryption enabled",
                        "Finding EC2 instances in production account"
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

        // Log intent if provided (helps with debugging agent reasoning)
        if let Some(ref intent) = input.intent {
            info!("🎯 Intent: {}", intent);
        }

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

/// Public API for executing JavaScript from webview
///
/// This function is called by the webview HTTP API to execute JavaScript code
/// with the same V8 bindings available to agent tools.
///
/// Returns the result as a JSON string for easy HTTP transport.
pub async fn execute_javascript_internal(code: String, intent: Option<String>) -> anyhow::Result<String> {
    use tokio::task;

    info!("🚀 execute_javascript_internal executing {} chars", code.len());

    if let Some(ref intent_str) = intent {
        info!("🎯 Intent: {}", intent_str);
    }

    // Execute in blocking context (V8 is sync)
    let result = task::spawn_blocking(move || {
        let config = RuntimeConfig::default();
        execute_with_error_handling(&code, &config)
    })
    .await
    .map_err(|e| anyhow::anyhow!("Task join error: {}", e))??;

    // Extract the result value from ToolResult
    if result.success {
        // ToolResult.content is a Value, try to extract just the "result" field
        if let Some(result_value) = result.content.get("result") {
            return Ok(serde_json::to_string(result_value)?);
        }
        // Fallback: return full content as JSON string
        Ok(serde_json::to_string(&result.content)?)
    } else {
        Err(anyhow::anyhow!("JavaScript execution failed: {}", result.error.unwrap_or_else(|| "Unknown error".to_string())))
    }
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
