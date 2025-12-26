# V8 Bindings Module - JavaScript Execution Engine

## Component Overview

Provides V8 JavaScript engine integration for agent tool execution.
Agents can execute JavaScript code with AWS API bindings in a sandboxed environment.

**Pattern**: Sandboxed script execution with native bindings
**External**: v8/rusty_v8 crate
**Purpose**: Safe JavaScript execution with AWS API access

---

## Module Structure

- `mod.rs` - Module exports
- `platform.rs` - V8 platform initialization (call once at startup)
- `runtime.rs` - V8Runtime, RuntimeConfig, ExecutionResult
- `console.rs` - JavaScript console API (log, warn, error)
- `types.rs` - V8 type conversions (Rust <-> JavaScript)
- `bindings/mod.rs` - Function binding registration
- `bindings/accounts.rs` - listAccounts() binding
- `bindings/regions.rs` - listRegions() binding
- `bindings/resources.rs` - queryResources() binding
- `bindings/cloudwatch_logs.rs` - queryCloudWatchLogEvents() binding
- `bindings/cloudtrail_events.rs` - getCloudTrailEvents() binding

---

## Key Types

### V8Runtime
JavaScript execution environment:
- Memory limit enforcement (256MB default)
- Timeout enforcement (30s default)
- Console output capture
- AWS API bindings

### RuntimeConfig
Configuration for V8Runtime:
- `memory_limit_mb`: Max heap size
- `timeout_seconds`: Execution timeout
- `enable_console`: Capture console output

### ExecutionResult
Result of JavaScript execution:
- `result`: JSON string of return value
- `console_output`: Captured console logs
- `execution_time_ms`: How long it took

---

## Available JavaScript APIs

### listAccounts()
Returns array of available AWS accounts.

### listRegions()
Returns array of available AWS regions.

### queryResources(resourceType, accountId, region)
Query AWS resources by type, account, and region.
Returns array of resource objects.

### queryCloudWatchLogEvents(logGroupName, startTime, endTime, filterPattern)
Query CloudWatch log events.

### getCloudTrailEvents(startTime, endTime, eventName)
Query CloudTrail audit events.

---

## Initialization

```rust
// Call once at application startup
initialize_v8_platform();

// Create runtime for execution
let config = RuntimeConfig::default();
let runtime = V8Runtime::new(config)?;

// Execute JavaScript
let result = runtime.execute(r#"
    const instances = await queryResources('AWS::EC2::Instance', 'acct', 'us-east-1');
    JSON.stringify(instances);
"#)?;
```

---

## Security Model

- Sandboxed execution (no file system access)
- Memory limits prevent DoS
- Timeout prevents infinite loops
- Only whitelisted AWS APIs exposed
- No network access except through bindings

---

**Last Updated**: 2025-12-22
**Status**: Accurately reflects v8_bindings/ module structure
