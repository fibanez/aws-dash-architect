# Code Execution Tool

V8-based JavaScript execution system enabling AI agents to solve AWS infrastructure problems through code rather than specialized tools.

## Core Functionality

**JavaScript Execution:**
- Isolated V8 sandbox for secure code execution
- 256MB memory limit per execution
- 30 second timeout enforcement
- Console output capture (log, error, warn, debug)
- Automatic JSON-based type conversion between Rust and JavaScript

**Key Features:**
- Fresh V8 isolate per execution (no state persistence)
- Comprehensive error handling (syntax, runtime, timeout)
- Rust function bindings exposed as JavaScript globals
- Automatic API documentation generation for LLM consumption
- Thread-safe execution with proper lifetime management

**Main Components:**
- **V8Runtime**: Core execution engine with isolate management
- **ExecuteJavaScriptTool**: Tool trait implementation for agent framework
- **ConsoleBuffers**: Output capture for console methods
- **Function Bindings**: Rust-to-JavaScript API bridge
- **Type Conversion**: JSON-based value conversion system

**Integration Points:**
- Agent Framework V2 for LLM-driven execution
- AWS SDK clients via function bindings
- Global credential system for AWS operations
- Tool registry for agent configuration

## Architecture

### 4-Layer Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    CODE EXECUTION SYSTEM                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Layer 1: V8 Platform & Runtime                                 │
│  ├─ Location: src/app/agent_framework/v8_bindings/             │
│  ├─ Files: platform.rs, runtime.rs                             │
│  └─ Purpose: V8 isolate management and execution               │
│                                                                  │
│  Layer 2: Type Conversion & Console                             │
│  ├─ Location: src/app/agent_framework/v8_bindings/             │
│  ├─ Files: types.rs, console.rs                                │
│  └─ Purpose: Rust ↔ JavaScript data conversion                 │
│                                                                  │
│  Layer 3: Function Bindings                                     │
│  ├─ Location: src/app/agent_framework/v8_bindings/bindings/    │
│  ├─ Files: accounts.rs, regions.rs, resources.rs, etc.         │
│  └─ Purpose: Expose Rust functions to JavaScript               │
│                                                                  │
│  Layer 4: Tool Implementation                                   │
│  ├─ Location: src/app/agent_framework/tools/                   │
│  ├─ File: execute_javascript.rs                                │
│  └─ Purpose: Agent framework integration                       │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Execution Flow

```
Agent → Tool → V8Runtime → Isolate → JavaScript Code
                             ↓
                    Function Bindings (Rust)
                             ↓
                    AWS SDK Clients
                             ↓
                    AWS Services
                             ↓
Results ← Tool ← ExecutionResult ← Console + Return Value
```

## Implementation Details

**Key Files:**
- `src/app/agent_framework/v8_bindings/platform.rs` - Global V8 platform initialization
- `src/app/agent_framework/v8_bindings/runtime.rs` - V8Runtime execution engine
- `src/app/agent_framework/v8_bindings/console.rs` - Console output capture
- `src/app/agent_framework/v8_bindings/types.rs` - Type conversion utilities
- `src/app/agent_framework/v8_bindings/bindings/mod.rs` - Function binding registry
- `src/app/agent_framework/tools/execute_javascript.rs` - Tool implementation

### Layer 1: V8 Platform & Runtime

**Global Platform Initialization** (`platform.rs`):
```rust
use once_cell::sync::OnceCell;

static V8_PLATFORM: OnceCell<()> = OnceCell::new();

pub fn initialize_v8_platform() -> &'static () {
    V8_PLATFORM.get_or_init(|| {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();
    })
}
```

Key decisions:
- **OnceCell**: Ensures single initialization across all threads
- **Default platform**: Uses V8's default platform with 0 worker threads (single-threaded)
- **Shared platform**: Platform shared across all isolates

**V8Runtime** (`runtime.rs`):
```rust
pub struct V8Runtime {
    isolate: v8::OwnedIsolate,
    context: v8::Global<v8::Context>,
    console_buffers: Option<ConsoleBuffers>,
    config: RuntimeConfig,
}

pub struct RuntimeConfig {
    pub timeout: Duration,           // Default: 30s
    pub max_heap_size_mb: usize,     // Default: 256MB
}

impl V8Runtime {
    pub fn new(config: RuntimeConfig) -> Result<Self> {
        // Ensure platform initialized
        initialize_v8_platform();

        // Create isolate with memory limits
        let params = v8::CreateParams::default()
            .heap_limits(0, config.max_heap_size_mb * 1024 * 1024);
        let mut isolate = v8::Isolate::new(params);

        // Create context
        let scope = &mut v8::HandleScope::new(&mut isolate);
        let context = v8::Context::new(scope, Default::default());
        let global_context = v8::Global::new(scope, context);

        Ok(Self {
            isolate,
            context: global_context,
            console_buffers: None,
            config,
        })
    }

    pub fn execute(&mut self, code: &str) -> Result<ExecutionResult> {
        let start_time = Instant::now();

        // Set up timeout termination
        let isolate_handle = self.isolate.thread_safe_handle();
        let timeout = self.config.timeout;
        std::thread::spawn(move || {
            std::thread::sleep(timeout);
            isolate_handle.terminate_execution();
        });

        // Execute in context scope
        let scope = &mut v8::HandleScope::new(&mut self.isolate);
        let context = v8::Local::new(scope, &self.context);
        let scope = &mut v8::ContextScope::new(scope, context);

        // Compile code
        let source = v8::String::new(scope, code).unwrap();
        let script = match v8::Script::compile(scope, source, None) {
            Some(script) => script,
            None => {
                return Ok(ExecutionResult {
                    success: false,
                    stderr: "Syntax error".to_string(),
                    ..Default::default()
                });
            }
        };

        // Execute script
        let result = match script.run(scope) {
            Some(result) => result,
            None => {
                return Ok(ExecutionResult {
                    success: false,
                    stderr: "Runtime error".to_string(),
                    ..Default::default()
                });
            }
        };

        // Convert result to JSON
        let result_json = from_v8_value(scope, result)?;

        Ok(ExecutionResult {
            success: true,
            result: Some(result_json),
            stdout: self.get_stdout(),
            stderr: String::new(),
            execution_time_ms: start_time.elapsed().as_millis() as u64,
        })
    }
}
```

**ExecutionResult**:
```rust
pub struct ExecutionResult {
    pub success: bool,
    pub result: Option<serde_json::Value>,
    pub stdout: String,
    pub stderr: String,
    pub execution_time_ms: u64,
}
```

### Layer 2: Type Conversion & Console

**Type Conversion** (`types.rs`):

Uses JSON as the bridge between Rust and JavaScript:

```rust
/// Convert Rust value to V8 value via JSON
pub fn to_v8_value<'s, T>(
    scope: &mut v8::HandleScope<'s>,
    value: &T,
) -> Result<v8::Local<'s, v8::Value>>
where
    T: serde::Serialize,
{
    let json = serde_json::to_value(value)?;
    let json_str = serde_json::to_string(&json)?;
    let v8_str = v8::String::new(scope, &json_str).unwrap();
    let parsed = v8::json::parse(scope, v8_str).unwrap();
    Ok(parsed)
}

/// Convert V8 value to Rust value via JSON
pub fn from_v8_value<T>(
    scope: &mut v8::HandleScope,
    value: v8::Local<v8::Value>,
) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    let json_str = v8::json::stringify(scope, value)
        .unwrap()
        .to_rust_string_lossy(scope);
    let result: T = serde_json::from_str(&json_str)?;
    Ok(result)
}
```

Key design decision: JSON-based conversion is simpler than manual field mapping and handles complex types automatically.

**Utility Functions**:
```rust
pub fn v8_string<'s>(scope: &mut v8::HandleScope<'s>, s: &str)
    -> Result<v8::Local<'s, v8::String>>

pub fn v8_null<'s>(scope: &mut v8::HandleScope<'s>)
    -> v8::Local<'s, v8::Value>

pub fn v8_undefined<'s>(scope: &mut v8::HandleScope<'s>)
    -> v8::Local<'s, v8::Value>

pub fn v8_error<'s>(scope: &mut v8::HandleScope<'s>, message: &str)
    -> v8::Local<'s, v8::Value>
```

**Console Buffers** (`console.rs`):
```rust
#[derive(Clone)]
pub struct ConsoleBuffers {
    stdout: Rc<RefCell<String>>,
    stderr: Rc<RefCell<String>>,
}

impl ConsoleBuffers {
    pub fn new() -> Self {
        Self {
            stdout: Rc::new(RefCell::new(String::new())),
            stderr: Rc::new(RefCell::new(String::new())),
        }
    }

    pub fn get_stdout(&self) -> String {
        self.stdout.borrow().clone()
    }

    pub fn get_stderr(&self) -> String {
        self.stderr.borrow().clone()
    }
}
```

**Console Binding Registration**:
```rust
pub fn register_console(
    scope: &mut v8::ContextScope<'_, '_, v8::HandleScope<'_>>,
    buffers: ConsoleBuffers,
) -> Result<()> {
    let global = scope.get_current_context().global(scope);

    // Create console object
    let console_obj = v8::Object::new(scope);

    // Register console.log
    let log_fn = create_console_fn(scope, buffers.clone(), "log")?;
    let log_name = v8_string(scope, "log")?;
    console_obj.set(scope, log_name.into(), log_fn.into());

    // Register console.error
    let error_fn = create_console_fn(scope, buffers.clone(), "error")?;
    let error_name = v8_string(scope, "error")?;
    console_obj.set(scope, error_name.into(), error_fn.into());

    // Similar for warn, debug...

    // Attach console to global
    let console_name = v8_string(scope, "console")?;
    global.set(scope, console_name.into(), console_obj.into());

    Ok(())
}
```

**V8 Callback Pattern**:
```rust
fn create_console_fn<'s>(
    scope: &mut v8::HandleScope<'s>,
    buffers: ConsoleBuffers,
    method: &str,
) -> Result<v8::Local<'s, v8::Function>> {
    // Store buffers in V8 External
    let external = v8::External::new(scope, Box::into_raw(Box::new(buffers)) as *mut _);

    // Create function with external data
    let function = v8::Function::builder(console_log_callback)
        .data(external.into())
        .build(scope)?;

    Ok(function)
}

fn console_log_callback(
    scope: &mut v8::HandleScope,
    args: v8::FunctionCallbackArguments,
    _retval: v8::ReturnValue,
) {
    // Extract buffers from external data
    let data = args.data();
    let external = v8::Local::<v8::External>::try_from(data).unwrap();
    let buffers_ptr = external.value() as *const ConsoleBuffers;
    let buffers = unsafe { &*buffers_ptr };

    // Concatenate all arguments
    let mut output = String::new();
    for i in 0..args.length() {
        if i > 0 {
            output.push(' ');
        }
        let arg = args.get(i);
        output.push_str(&arg.to_rust_string_lossy(scope));
    }

    // Append to stdout
    buffers.stdout.borrow_mut().push_str(&output);
    buffers.stdout.borrow_mut().push('\n');
}
```

### Layer 3: Function Bindings

**Binding Registry** (`bindings/mod.rs`):
```rust
pub fn register_bindings(
    scope: &mut v8::ContextScope<'_, '_, v8::HandleScope<'_>>,
) -> Result<()> {
    accounts::register(scope)?;
    regions::register(scope)?;
    resources::register(scope)?;
    cloudwatch_logs::register(scope)?;
    cloudtrail_events::register(scope)?;
    Ok(())
}

pub fn get_api_documentation() -> String {
    let mut docs = String::new();
    docs.push_str("# Available JavaScript APIs\n\n");
    docs.push_str("## Account Management\n\n");
    docs.push_str(&accounts::get_documentation());
    docs.push_str("\n## Region Management\n\n");
    docs.push_str(&regions::get_documentation());
    // ... other categories
    docs
}
```

**Example Binding** (`bindings/accounts.rs`):
```rust
use lazy_static::lazy_static;
use std::sync::RwLock;

// Global AWS identity (credentials)
lazy_static! {
    static ref GLOBAL_AWS_IDENTITY: RwLock<Option<AwsIdentity>> = RwLock::new(None);
}

pub fn set_global_aws_identity(identity: AwsIdentity) {
    *GLOBAL_AWS_IDENTITY.write().unwrap() = Some(identity);
}

pub fn register(
    scope: &mut v8::ContextScope<'_, '_, v8::HandleScope<'_>>,
) -> Result<()> {
    let global = scope.get_current_context().global(scope);

    // Create listAccounts function
    let fn_name = v8_string(scope, "listAccounts")?;
    let function = v8::Function::new(scope, list_accounts_callback)
        .ok_or_else(|| anyhow!("Failed to create function"))?;

    // Attach to global
    global.set(scope, fn_name.into(), function.into());

    Ok(())
}

fn list_accounts_callback(
    scope: &mut v8::HandleScope,
    _args: v8::FunctionCallbackArguments,
    mut retval: v8::ReturnValue,
) {
    // Get global identity
    let identity_guard = GLOBAL_AWS_IDENTITY.read().unwrap();
    let identity = match identity_guard.as_ref() {
        Some(id) => id,
        None => {
            let error = v8_error(scope, "No AWS identity configured");
            retval.set(error);
            return;
        }
    };

    // Get accounts
    let accounts: Vec<Account> = identity.accounts.clone();

    // Convert to V8 value
    match to_v8_value(scope, &accounts) {
        Ok(value) => retval.set(value),
        Err(e) => {
            let error = v8_error(scope, &format!("Failed to convert accounts: {}", e));
            retval.set(error);
        }
    }
}

pub fn get_documentation() -> String {
    r#"
### listAccounts()

Returns all configured AWS accounts from Identity Center.

**Signature:**
```typescript
function listAccounts(): Account[]

interface Account {
  id: string;          // AWS account ID (e.g., "123456789012")
  name: string;        // Human-readable name
  alias: string | null; // Account alias if set
  email: string | null; // Email associated with account
}
```

**JSON Schema:**
```json
{
  "type": "array",
  "items": {
    "type": "object",
    "properties": {
      "id": { "type": "string" },
      "name": { "type": "string" },
      "alias": { "type": ["string", "null"] },
      "email": { "type": ["string", "null"] }
    },
    "required": ["id", "name"]
  }
}
```

**Example:**
```javascript
const accounts = listAccounts();
console.log(`Found ${accounts.length} accounts`);

// Filter production accounts
const prodAccounts = accounts.filter(a => a.name.includes('prod'));
console.log(`Production accounts: ${prodAccounts.length}`);

// Get specific account
const account = accounts.find(a => a.id === '123456789012');
if (account) {
    console.log(`Account: ${account.name}`);
}
```

**Returns:** Array of account objects

**Errors:** Throws error if no AWS identity configured
    "#.to_string()
}
```

**Available Bindings:**

1. **Account Management** (`bindings/accounts.rs`):
   - `listAccounts()` - Get all configured accounts

2. **Region Management** (`bindings/regions.rs`):
   - `listRegions(region_hint: string)` - Get available regions

3. **Resource Queries** (`bindings/resources.rs`):
   - `queryResources(options)` - Query AWS resources with flexible filtering
   - `listBookmarks()` - Get saved resource bookmarks
   - `queryBookmarks(options)` - Query resources from a bookmark

4. **CloudWatch Logs** (`bindings/cloudwatch_logs.rs`):
   - `listLogGroups(account_id, region)` - Get log groups
   - `getLogEvents(account_id, region, log_group, log_stream)` - Query events

5. **CloudTrail Events** (`bindings/cloudtrail_events.rs`):
   - `lookupEvents(account_id, region, filters)` - Query CloudTrail

**Resource Query API Details:**

The `queryResources()` function supports detail levels for performance optimization:

```typescript
interface QueryOptions {
  accounts: string[] | null;      // Filter by account IDs, null for all
  regions: string[] | null;       // Filter by regions, null for all
  resourceTypes: string[];        // Required: AWS resource types
  detail: "count" | "summary" | "tags" | "full";  // Detail level
}

interface QueryResult {
  status: "success" | "partial" | "error";
  data: Resource[] | null;        // null for "count" mode
  count: number;                  // Total resources found
  warnings: string[];             // Non-fatal issues
  errors: string[];               // Fatal issues
  detailsLoaded: boolean;         // Phase 2 enrichment completed
  detailsPending: boolean;        // Phase 2 still running
}
```

**Detail Levels:**
- `"count"` - Returns only the count (fastest, minimal context)
- `"summary"` - Basic info: id, name, type, account, region
- `"tags"` - Summary + tags array (for tag-based filtering)
- `"full"` - Complete data including `detailedProperties` (waits for Phase 2)

**The `detailedProperties` Field:**

Resources that support Phase 2 enrichment have a `detailedProperties` field containing security-relevant data:
- S3 buckets: bucket policies, encryption, versioning
- Lambda functions: function configuration, environment variables
- IAM roles/users: inline policies, attached policies
- KMS keys: key policies, rotation status

Non-enrichable resources (EC2, VPC) have `detailedProperties: null`.

**Error Codes:**

Errors in the `errors` array have specific codes:
- `AccessDenied` - IAM permissions insufficient
- `InvalidToken` - Region not enabled (opt-in regions like me-south-1)
- `OptInRequired` - Region requires explicit opt-in in AWS Console
- `Timeout` - Network timeout (retryable)
- `RateLimitExceeded` - API throttled (retryable)

**Global Services:**

For global services (S3, IAM, Route53, CloudFront, Organizations), the region parameter has no filtering effect. The system queries once per account automatically. See [Resource Explorer System](resource-explorer-system.md#global-services) for details

### Layer 4: Tool Implementation

**ExecuteJavaScriptTool** (`tools/execute_javascript.rs`):
```rust
pub struct ExecuteJavaScriptTool {
    config: RuntimeConfig,
}

impl ExecuteJavaScriptTool {
    pub fn new() -> Self {
        Self {
            config: RuntimeConfig::default(),
        }
    }

    pub fn with_config(config: RuntimeConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl Tool for ExecuteJavaScriptTool {
    fn name(&self) -> &str {
        "execute_javascript"
    }

    fn description(&self) -> &str {
        "Execute JavaScript code in an isolated V8 sandbox"
    }

    fn parameters_json_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "code": {
                    "type": "string",
                    "description": "JavaScript code to execute. Use last expression as return value."
                }
            },
            "required": ["code"]
        })
    }

    async fn execute(
        &self,
        input: Option<serde_json::Value>,
        _agent_logger: Option<Arc<AgentLogger>>,
    ) -> Result<ToolResult> {
        // Parse input
        let input: ExecuteJavaScriptInput = serde_json::from_value(
            input.ok_or_else(|| anyhow!("No input provided"))?
        )?;

        // Validate code
        if input.code.trim().is_empty() {
            return Ok(ToolResult::error("Code parameter cannot be empty"));
        }

        // Create V8 runtime
        let mut runtime = V8Runtime::new(self.config.clone())?;

        // Register console bindings
        let console_buffers = ConsoleBuffers::new();
        runtime.register_console(console_buffers.clone())?;

        // Register function bindings
        runtime.register_bindings()?;

        // Execute JavaScript
        let execution_result = runtime.execute(&input.code)?;

        // Format result for agent
        Ok(format_execution_result(execution_result))
    }
}

fn format_execution_result(result: ExecutionResult) -> ToolResult {
    if result.success {
        let output = format!(
            "Execution completed successfully in {}ms\n\n\
             === Result ===\n\
             {}\n\n\
             === Console Output ===\n\
             {}",
            result.execution_time_ms,
            result.result
                .map(|v| serde_json::to_string_pretty(&v).unwrap_or_else(|_| "null".to_string()))
                .unwrap_or_else(|| "undefined".to_string()),
            if result.stdout.is_empty() {
                "(no output)".to_string()
            } else {
                result.stdout
            }
        );
        ToolResult::success(output)
    } else {
        let error_msg = format!(
            "Execution failed after {}ms\n\n\
             === Error ===\n\
             {}\n\n\
             === Console Output (before error) ===\n\
             {}",
            result.execution_time_ms,
            result.stderr,
            if result.stdout.is_empty() {
                "(no output)".to_string()
            } else {
                result.stdout
            }
        );
        ToolResult::error(&error_msg)
    }
}
```

## Key Design Decisions

**1. Why V8 Instead of Other JavaScript Engines?**
- **Mature**: V8 is battle-tested in Chrome and Node.js
- **Performance**: JIT compilation for fast execution
- **Isolation**: Strong sandbox model with memory limits
- **Rust Bindings**: rusty_v8 provides comprehensive bindings
- **Debugging**: Excellent error messages and stack traces

**2. Why JSON-Based Type Conversion?**
- **Simplicity**: One conversion path for all types
- **Automatic**: Works with any Serialize/Deserialize type
- **Familiar**: JSON is well-understood by both Rust and JavaScript
- **Flexible**: Handles complex nested structures automatically

Alternative considered: Manual field mapping (too complex, error-prone)

**3. Why Single Tool Instead of Multiple Specialized Tools?**
- **Fewer Round Trips**: Complex logic in single tool call
- **Better Context**: LLM maintains state in JavaScript
- **More Flexible**: LLM can write any logic, not limited to predefined operations
- **Easier Debugging**: Full execution trace visible in code

**4. Why Fresh Isolate Per Execution?**
- **Security**: No state leakage between executions
- **Simplicity**: No need to manage isolate lifecycle
- **Reliability**: Crashed isolates don't affect future executions
- **Performance**: V8 isolate creation is fast (~5-10ms)

**5. Why Global Credentials Instead of Per-Execution?**
- **Agent Context**: Credentials tied to agent lifecycle
- **Performance**: Avoid repeated Identity Center calls
- **Simplicity**: Single set of credentials per agent session

## Performance Characteristics

**Execution Times (Typical):**
- V8 platform init: ~50ms (one-time startup cost)
- Isolate creation: ~5-10ms
- Console binding registration: ~1ms
- Function binding registration: ~2-3ms per category
- Simple code execution: ~1-5ms
- Complex code execution: ~10-100ms
- AWS API call via binding: ~100-500ms (network latency)

**Memory Usage:**
- V8 platform: ~10MB
- Isolate base: ~5MB
- Heap limit: 256MB (configurable)
- Typical usage: ~20-50MB per execution

**Concurrency:**
- Multiple agents can execute JavaScript simultaneously
- Each execution has isolated V8 isolate (no contention)
- Global credentials protected by RwLock (minimal contention)

## Developer Notes

**Adding New JavaScript Functions:**

See [Adding New JavaScript APIs](agent-framework-v2.md#adding-new-javascript-apis) in Agent Framework V2 documentation.

**Testing Code Execution:**

```rust
#[test]
fn test_javascript_execution() {
    let _ = initialize_v8_platform();

    let tool = ExecuteJavaScriptTool::new();
    let input = json!({
        "code": "const x = 5 + 3; x"
    });

    let result = tool.execute(Some(input), None).await.unwrap();

    assert!(result.success);
    assert!(result.content.contains("8"));
}
```

**Debugging V8 Execution:**

1. **Enable V8 tracing**:
   ```bash
   export V8_FLAGS="--trace-opt --trace-deopt"
   ```

2. **Check console output**:
   ```javascript
   console.log('Debug:', JSON.stringify(value, null, 2));
   ```

3. **Test in isolation**:
   ```rust
   let mut runtime = V8Runtime::new(RuntimeConfig::default())?;
   let result = runtime.execute("console.log('test'); 42")?;
   println!("Result: {:?}", result);
   ```

**Common Pitfalls:**

1. **Scope Lifetime Issues**:
   ```rust
   // ❌ WRONG - scope doesn't live long enough
   let value = {
       let scope = &mut v8::HandleScope::new(&mut isolate);
       v8::String::new(scope, "test").unwrap()
   }; // scope dropped here!

   // ✅ CORRECT - scope lives long enough
   let scope = &mut v8::HandleScope::new(&mut isolate);
   let value = v8::String::new(scope, "test").unwrap();
   ```

2. **Forgetting Platform Initialization**:
   ```rust
   // ❌ WRONG - platform not initialized
   let isolate = v8::Isolate::new(params); // Panics!

   // ✅ CORRECT - initialize first
   initialize_v8_platform();
   let isolate = v8::Isolate::new(params);
   ```

3. **Return Value vs Last Expression**:
   ```javascript
   // ❌ WRONG - no return statement in V8
   function test() {
       return 42;
   }
   test()

   // ✅ CORRECT - use last expression
   const test = () => 42;
   test()
   ```

**Security Considerations:**

- **Sandbox**: V8 isolates provide memory isolation (no file system access)
- **Timeout**: Prevents infinite loops and long-running code
- **Memory Limit**: Prevents memory exhaustion attacks
- **No eval**: LLM writes code directly, not evaluated from strings
- **Credential Isolation**: Global credentials cleared when agent destroyed

## Extension Points

**Custom Type Conversions:**

```rust
// Add custom conversion for complex types
impl ToV8Value for MyComplexType {
    fn to_v8_value<'s>(&self, scope: &mut v8::HandleScope<'s>)
        -> Result<v8::Local<'s, v8::Value>>
    {
        // Custom conversion logic
    }
}
```

**Custom Console Methods:**

```rust
// Add console.trace, console.table, etc.
pub fn register_console_trace(
    scope: &mut v8::ContextScope,
    buffers: ConsoleBuffers,
) -> Result<()> {
    // Similar pattern to console.log
}
```

**Custom Error Handling:**

```rust
pub enum ExecutionError {
    SyntaxError(String),
    RuntimeError(String),
    Timeout,
    MemoryLimit,
}

impl V8Runtime {
    pub fn execute_with_detailed_errors(&mut self, code: &str)
        -> Result<ExecutionResult, ExecutionError>
    {
        // Enhanced error reporting
    }
}
```

## Related Documentation

- [Agent Framework V2](agent-framework-v2.md) - Agent system using code execution
- [AWS Data Plane Integration](aws-data-plane-integration-guide.md) - Adding AWS service bindings
- [Credential Management](credential-management.md) - AWS Identity Center integration
