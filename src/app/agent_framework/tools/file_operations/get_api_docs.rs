//! Get API Docs Tool - Get dashApp API documentation
//!
//! This tool provides the complete dashApp API reference to Tool Builder agents,
//! helping them understand what APIs are available when building tools.

#![warn(clippy::all, rust_2018_idioms)]

use anyhow::Result;
use async_trait::async_trait;
use serde::Serialize;
use serde_json::Value;
use stood::tools::{Tool, ToolError, ToolResult};

/// Tool for getting dashApp API documentation
#[derive(Debug, Clone)]
pub struct GetApiDocsTool;

#[derive(Debug, Serialize)]
struct ApiDocsResult {
    api_reference: String,
}

impl GetApiDocsTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GetApiDocsTool {
    fn default() -> Self {
        Self::new()
    }
}

const DASH_APP_API_DOCS: &str = r#"
# dashApp API Reference

The `window.dashApp` API provides Dash Tools with access to AWS resource data and UI integration.

## Account & Region APIs

### listAccounts()
Get all configured AWS accounts.

**Returns:** `Array<{ id: string, name: string, alias: string|null, email: string|null }>`

**Example:**
```javascript
const accounts = dashApp.listAccounts();
console.log(`Found ${accounts.length} accounts`);
```

### listRegions()
Get all AWS regions with their codes and names.

**Returns:** `Array<{ code: string, name: string }>`

**Example:**
```javascript
const regions = dashApp.listRegions();
const usEast1 = regions.find(r => r.code === 'us-east-1');
```

## Resource Query APIs

### loadCache(params)
Load AWS resources into cache. Returns counts only (not full resources) to minimize context usage.

**Parameters:**
- `accounts`: `string[]|null` - Account IDs to query (null = common regions only)
- `regions`: `string[]|null` - Region codes (null = default regions)
- `resourceTypes`: `string[]` - CloudFormation resource types (REQUIRED)

**Returns:**
```javascript
{
  status: "success"|"partial"|"error",
  countByScope: { "account:region:type": count },
  totalCount: number,
  warnings: Array<{ account, region, message }>,
  errors: Array<{ account, region, code, message }>,
  accountsQueried: string[],
  regionsQueried: string[],
  loadTimestampUtc: string
}
```

**Example:**
```javascript
const result = await dashApp.loadCache({
  accounts: accounts.map(a => a.id),
  regions: ['us-east-1', 'us-west-2'],
  resourceTypes: ['AWS::S3::Bucket', 'AWS::EC2::Instance']
});
console.log(`Loaded ${result.totalCount} resources`);
```

### getResourceSchema(resourceType)
Get ONE example resource to understand available properties.

**Parameters:**
- `resourceType`: `string` - CloudFormation type (e.g., "AWS::EC2::Instance")

**Returns:**
```javascript
{
  status: "success"|"not_found",
  resourceType: string,
  exampleResource: {
    resourceId: string,
    displayName: string,
    accountId: string,
    region: string,
    properties: {...},
    tags: [...],
    status: string
  },
  cacheStats: { totalCount, accountCount, regionCount },
  message: string (if not_found)
}
```

**Example:**
```javascript
const schema = await dashApp.getResourceSchema('AWS::S3::Bucket');
console.log('Available properties:', Object.keys(schema.exampleResource.properties));
```

### queryCachedResources(params)
Query actual resources from cache for filtering and analysis.

**Parameters:**
- `accounts`: `string[]|null` - Account IDs to filter (null = all cached)
- `regions`: `string[]|null` - Region codes to filter (null = all cached)
- `resourceTypes`: `string[]` - Resource types to query (REQUIRED)

**Returns:**
```javascript
{
  status: "success"|"not_found",
  resources: Array<ResourceEntry>,
  count: number,
  accountsWithData: string[],
  regionsWithData: string[],
  resourceTypesFound: string[],
  message: string (if not_found)
}
```

**Example:**
```javascript
const resources = await dashApp.queryCachedResources({
  accounts: [accounts[0].id],
  regions: ['us-east-1'],
  resourceTypes: ['AWS::S3::Bucket']
});
console.log(`Found ${resources.count} S3 buckets`);
```

## UI Integration

### showInExplorer(config)
Open the Resource Explorer window with specific configuration.

**Parameters:**
```javascript
{
  accounts: string[],
  regions: string[],
  resourceTypes: string[],
  grouping: { type: "ByAccount"|"ByRegion"|"ByResourceType"|"ByTag", key?: string },
  tagFilters: { operator: "And"|"Or", filters: [...] },
  searchFilter: string,
  title: string
}
```

**Returns:**
```javascript
{
  status: "success"|"error",
  message: string,
  resourcesDisplayed: number
}
```

**Example:**
```javascript
await dashApp.showInExplorer({
  accounts: [accountId],
  regions: ['us-east-1'],
  resourceTypes: ['AWS::S3::Bucket'],
  grouping: { type: 'ByAccount' },
  title: 'Production S3 Buckets'
});
```

## Logging & Events

### queryCloudWatchLogEvents(params)
Query CloudWatch Logs for analysis and monitoring.

**Parameters:**
```javascript
{
  logGroupName: string,
  accountId: string,
  region: string,
  startTime?: number,      // Unix milliseconds
  endTime?: number,        // Unix milliseconds
  filterPattern?: string,
  limit?: number,
  logStreamNames?: string[],
  startFromHead?: boolean
}
```

**Returns:**
```javascript
{
  events: Array<{
    timestamp: number,
    message: string,
    ingestionTime: number,
    logStreamName: string
  }>,
  nextToken: string|null,
  totalEvents: number,
  statistics: {
    bytesScanned: number,
    recordsMatched: number,
    recordsScanned: number
  }
}
```

### getCloudTrailEvents(params)
Query CloudTrail events for governance and compliance.

**Parameters:**
```javascript
{
  accountId: string,
  region: string,
  startTime?: number,
  endTime?: number,
  lookupAttributes?: Array<{
    attributeKey: string,
    attributeValue: string
  }>,
  maxResults?: number
}
```

**Returns:**
```javascript
{
  events: Array<{
    eventId: string,
    eventName: string,
    eventTime: number,
    eventSource: string,
    username: string,
    resources: Array<{
      resourceType: string,
      resourceName: string
    }>,
    errorCode?: string
  }>,
  nextToken: string|null,
  totalEvents: number
}
```

## Persistence

### saveCurrentApp(params)
Save this tool to persistent storage for later use.

**Parameters:**
```javascript
{
  name: string,
  description?: string,
  folder_id?: string
}
```

**Returns:**
```javascript
{
  status: "success"|"error",
  tool_id: string,
  message: string
}
```

**Example:**
```javascript
const result = await dashApp.saveCurrentApp({
  name: 'S3 Bucket Explorer',
  description: 'Browse and analyze S3 buckets across accounts'
});
console.log(`Saved tool with ID: ${result.tool_id}`);
```

## Resource Query Workflow

**Best practice for efficient resource queries:**

1. **Load Cache** - Populate cache with resource counts
2. **Get Schema** - Understand available properties
3. **Query Resources** - Filter and analyze specific resources

**Example workflow:**
```javascript
// Step 1: Load cache
const loadResult = await dashApp.loadCache({
  accounts: accounts.map(a => a.id),
  regions: ['us-east-1'],
  resourceTypes: ['AWS::EC2::SecurityGroup']
});

// Step 2: Get schema to understand structure
const schema = await dashApp.getResourceSchema('AWS::EC2::SecurityGroup');
console.log('Properties:', Object.keys(schema.exampleResource.properties));

// Step 3: Query specific resources
const resources = await dashApp.queryCachedResources({
  accounts: null,  // all cached
  regions: null,   // all cached
  resourceTypes: ['AWS::EC2::SecurityGroup']
});

// Step 4: Filter and display
const openSGs = resources.resources.filter(sg => {
  return sg.properties.IpPermissions?.some(rule =>
    rule.IpRanges?.some(range => range.CidrIp === '0.0.0.0/0')
  );
});
console.log(`Found ${openSGs.length} security groups open to internet`);
```

## Supported Resource Types

AWS Dash supports 93 AWS services and 183 resource types. Common examples:

- Compute: AWS::EC2::Instance, AWS::Lambda::Function, AWS::ECS::Service
- Storage: AWS::S3::Bucket, AWS::EBS::Volume, AWS::EFS::FileSystem
- Database: AWS::RDS::DBInstance, AWS::DynamoDB::Table
- Networking: AWS::EC2::VPC, AWS::EC2::SecurityGroup, AWS::EC2::Subnet
- IAM: AWS::IAM::Role, AWS::IAM::Policy, AWS::IAM::User

Use `getResourceSchema()` to explore properties for any resource type.
"#;

#[async_trait]
impl Tool for GetApiDocsTool {
    fn name(&self) -> &str {
        "get_api_docs"
    }

    fn description(&self) -> &str {
        "Get complete dashApp API reference documentation"
    }

    fn parameters_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {}
        })
    }

    async fn execute(
        &self,
        _parameters: Option<Value>,
        _agent_context: Option<&stood::agent::AgentContext>,
    ) -> Result<ToolResult, ToolError> {
        let result = ApiDocsResult {
            api_reference: DASH_APP_API_DOCS.to_string(),
        };

        match serde_json::to_value(result) {
            Ok(json) => Ok(ToolResult::success(json)),
            Err(e) => Ok(ToolResult::error(format!(
                "Failed to serialize API docs: {}",
                e
            ))),
        }
    }
}
