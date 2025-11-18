# CODE EXECUTION TOOL - MILESTONE 5: AGENT INTEGRATION

**Created**: 2025-01-14
**Last Updated**: 2025-01-14
**Status**: Not Started
**Priority**: P0 - Blocking
**Estimated Duration**: 1-2 days

---

## Overview

Integrate the `ExecuteJavaScriptTool` with the OrchestrationAgent, replacing all existing tools. This milestone transforms the agent from a multi-tool system to a code-execution-only system where all capabilities are exposed through JavaScript APIs.

### Goals

1. **Remove Existing Tools**: Remove all 8 existing tools from OrchestrationAgent
2. **Remove Task Agent System**: Remove create_task tool and task agent spawning logic
3. **Register execute_javascript**: Add execute_javascript as the sole tool
4. **Update Agent Prompt**: Document JavaScript APIs for LLM
5. **Clean Up Imports**: Remove unused tool imports and dependencies

### Key Principles

- **Simplification**: Single tool, single pattern for all operations
- **Abstraction**: JavaScript APIs hide AWS/infrastructure complexity
- **LLM-Friendly**: Clear documentation in prompt with examples
- **MVP Focus**: Keep skill system code but don't use it yet
- **No Breaking Changes**: Keep UI and logging infrastructure intact

---

## Required Reading

Before implementing, review these documents:

1. **CODE_EXECUTION_TOOL.md** - Overall architecture and DECISION 14 (Agent Integration)
2. **CODE_EXECUTION_TOOL_PART_5.md** - ExecuteJavaScriptTool API and documentation
3. **src/app/agent_framework/agents/orchestration_agent.rs** - Current implementation

---

## Current State Analysis

### Existing Tools (to be removed)

Currently registered in orchestration_agent.rs:339-348:

1. `create_task_tool()` - Spawns task agents (REMOVE - no more sub-agents)
2. `todo_write_tool()` - Task tracking (KEEP for now - helpful for planning)
3. `todo_read_tool()` - Read tasks (KEEP with todo_write)
4. `aws_find_account_tool()` - Find AWS accounts (REMOVE - will use listAccounts() in JS)
5. `aws_find_region_tool()` - Find regions (REMOVE - will abstract regions in bindings)
6. `invoke_skill_tool()` - Load skills (REMOVE for MVP - may return later)
7. `read_file_tool()` - Read files (REMOVE - not needed for MVP)
8. `list_directory_tool()` - List directories (REMOVE - not needed for MVP)

### Tools to Keep

- **execute_javascript** (NEW) - The only tool for infrastructure operations
- **todo_write** (KEEP) - Still useful for multi-step task planning
- **todo_read** (KEEP) - Query task state

### Rationale for Keeping TodoWrite/TodoRead

The todo tools don't conflict with the code execution model:
- They help the LLM plan and track multi-step workflows
- They don't compete with execute_javascript (different purposes)
- They reduce cognitive load on the LLM by externalizing task state
- They provide user visibility into agent's planning process

---

## Sub-Milestone 5.1: Remove Existing Tools and Imports

**Estimated Effort**: 1 hour
**Priority**: P0 - Blocking
**Status**: Not Started

### Goal

Clean up tool registration and remove unused imports from orchestration_agent.rs.

### Implementation

**File**: `src/app/agent_framework/agents/orchestration_agent.rs`

**Step 1**: Update tool registration (line ~339)

```rust
// Before
.tools(vec![
    create_task_tool(),
    todo_write_tool(),
    todo_read_tool(),
    aws_find_account_tool(),
    aws_find_region_tool(),
    invoke_skill_tool(),
    read_file_tool(),
    list_directory_tool(),
]);

// After
.tools(vec![
    execute_javascript_tool(),  // NEW: Only infrastructure tool
    todo_write_tool(),           // KEEP: Planning/tracking
    todo_read_tool(),            // KEEP: Planning/tracking
]);
```

**Step 2**: Update imports (lines ~17-21)

```rust
// Before
use crate::app::agent_framework::{
    aws_find_account_tool, aws_find_region_tool, create_task_tool, get_global_skill_manager,
    init_agent_debug_logger, invoke_skill_tool, list_directory_tool, log_agent_debug_event,
    read_file_tool, set_global_aws_credentials, todo_read_tool, todo_write_tool, AgentDebugEvent,
};

// After
use crate::app::agent_framework::{
    execute_javascript_tool, get_global_skill_manager,
    init_agent_debug_logger, log_agent_debug_event,
    set_global_aws_credentials, todo_read_tool, todo_write_tool, AgentDebugEvent,
};
```

**Step 3**: Update module exports in `src/app/agent_framework/mod.rs`

Ensure `execute_javascript_tool` is exported from tools module:

```rust
pub use tools::{
    ExecuteJavaScriptTool,
    // ... other tools
};

// Helper function to create execute_javascript tool
pub fn execute_javascript_tool() -> ExecuteJavaScriptTool {
    ExecuteJavaScriptTool::new()
}
```

### Verification

- [ ] Code compiles without errors
- [ ] No unused import warnings for removed tools
- [ ] execute_javascript_tool() is available in orchestration_agent.rs

### Acceptance Criteria

- [ ] Tool registration uses only execute_javascript, todo_write, todo_read
- [ ] All removed tool imports cleaned up
- [ ] No compilation errors or warnings
- [ ] Module exports include execute_javascript_tool helper

---

## Sub-Milestone 5.2: Update Agent System Prompt

**Estimated Effort**: 2 hours
**Priority**: P0 - Blocking
**Status**: Not Started

### Goal

Replace the current agent prompt with one that emphasizes JavaScript code execution and documents available APIs.

### Implementation

**File**: `src/app/agent_framework/agents/orchestration_agent.rs`

Update `create_system_prompt()` method (lines 48-230):

```rust
pub fn create_system_prompt() -> String {
    let base_prompt = r#"# AWS Infrastructure Agent

You are an agent that solves AWS infrastructure problems by writing JavaScript code.

## Core Philosophy: Code-First Problem Solving

Instead of using many separate tools, you have ONE powerful tool: `execute_javascript`.
All AWS operations, data processing, and logic are performed by writing JavaScript code
that calls bound Rust functions.

🔴 **CRITICAL**: ALWAYS present tool results to the user. Never end your turn after calling a tool without summarizing what was found.

## Extended Thinking: Plan Before Coding

Before writing code, analyze:
1. **Problem Understanding**: What is the user asking for?
2. **Data Needed**: What AWS resources/accounts/data do I need?
3. **API Selection**: Which available JavaScript APIs will I use?
4. **Code Structure**: How will I process and present the results?
5. **Error Handling**: What could go wrong? How will I handle it?

## How It Works

1. **You write JavaScript code** using available APIs (documented below)
2. **execute_javascript runs your code** in a secure V8 sandbox
3. **Results returned** with stdout/stderr/return value
4. **You analyze results** and present findings to user
5. **Iterate if needed** - write new code based on results

## Authentication & Configuration

🌟 **NO CREDENTIALS NEEDED** - All infrastructure access is handled internally:
- AWS credentials: Configured automatically (via Identity Center)
- Regions: Managed internally
- Account selection: Use listAccounts() to see available accounts

Your job is to write logic and process data, not manage infrastructure credentials.

## Available JavaScript APIs

### Account Management

```javascript
/**
 * List all configured AWS accounts
 *
 * Returns an array of account objects. No credentials needed.
 *
 * @returns {Account[]} Array of account objects
 *
 * @example
 * const accounts = listAccounts();
 * console.log(`Found ${accounts.length} accounts`);
 * accounts.forEach(acc => console.log(`${acc.name}: ${acc.id}`));
 *
 * @example
 * // Filter by name
 * const prodAccounts = listAccounts().filter(a => a.name.includes('prod'));
 * console.log(`Production accounts: ${prodAccounts.length}`);
 */
function listAccounts(): Account[];

interface Account {
  id: string;          // AWS account ID (e.g., "123456789012")
  name: string;        // Human-readable name
  alias: string | null; // Account alias if set
  email: string | null; // Email associated with account
}
```

### Console Functions

```javascript
/**
 * Log messages to stdout (visible in tool results)
 */
console.log(...args);   // Info messages
console.error(...args); // Error messages (goes to stderr)
console.warn(...args);  // Warning messages
console.debug(...args); // Debug messages
```

## JavaScript Code Patterns

### Return Values

Use the last expression as the return value:

```javascript
// Simple values
const accounts = listAccounts();
accounts.length  // Returns the number

// Complex objects - use JSON.stringify
const result = {
    total: accounts.length,
    names: accounts.map(a => a.name)
};
JSON.stringify(result)  // Returns as JSON
```

### Console Output

Use console.log for intermediate results:

```javascript
const accounts = listAccounts();
console.log(`Found ${accounts.length} accounts`);

const prodAccounts = accounts.filter(a => a.alias === 'prod');
console.log(`Production accounts: ${prodAccounts.length}`);

prodAccounts  // Return the filtered list
```

### Error Handling

Check for edge cases:

```javascript
const accounts = listAccounts();

if (accounts.length === 0) {
    console.error('No accounts found');
    null  // Return null to indicate no results
} else {
    accounts
}
```

## Workflow Example

**User**: "List all accounts"

**Your Response**:
```
Let me retrieve all configured AWS accounts:

<execute_javascript>
{
  "code": "const accounts = listAccounts();\nconsole.log(`Found ${accounts.length} accounts`);\nJSON.stringify(accounts)"
}
</execute_javascript>
```

**Tool Result**:
```
Execution completed successfully in 5ms

=== Result ===
[
  {"id": "123456789012", "name": "Production", "alias": "prod", "email": "prod@example.com"},
  {"id": "234567890123", "name": "Development", "alias": "dev", "email": "dev@example.com"}
]

=== Console Output ===
Found 2 accounts
```

**Your Response to User**:
"Found 2 AWS accounts:
1. Production (123456789012) - prod@example.com
2. Development (234567890123) - dev@example.com"

## Task Planning Tool (Optional)

For multi-step workflows (3+ steps), use TodoWrite to track progress:

```
TodoWrite({
  todos: [
    { content: "List all accounts", status: "in_progress", activeForm: "Listing accounts" },
    { content: "Filter production accounts", status: "pending", activeForm: "Filtering production accounts" },
    { content: "Present results", status: "pending", activeForm: "Presenting results" }
  ]
})
```

## Error Recovery

If your code fails:
1. **Read the error message** carefully (in stderr)
2. **Understand what went wrong** (syntax error, logic error, etc.)
3. **Write corrected code** and try again
4. **Explain to user** what the issue was and how you fixed it

## Security & Limits

- **Memory**: 256MB limit per execution
- **Timeout**: 30 second limit per execution
- **Isolation**: Each execution is fresh (no state persistence between calls)
- **Sandbox**: No file system access, no network access - only bound functions

## Guidelines

1. **Be concise**: Keep responses under 4 lines after presenting tool results
2. **Always use real data**: Call listAccounts() to get actual account IDs (never use placeholders like "123456789012")
3. **Present results**: Always summarize tool results for the user
4. **Write clean code**: Use clear variable names, add comments for complex logic
5. **Test incrementally**: Start with simple queries, build up complexity
6. **Handle errors gracefully**: Check for empty results, validate data

"#;

    base_prompt.to_string()
}
```

### Acceptance Criteria

- [ ] Prompt emphasizes code-first problem solving
- [ ] JavaScript APIs documented with TypeScript-style signatures
- [ ] Examples show correct usage patterns
- [ ] Authentication abstraction explained (no credentials needed)
- [ ] Error handling guidance provided
- [ ] Console output patterns documented
- [ ] Return value patterns documented
- [ ] TodoWrite mentioned but not emphasized (optional)

---

## Sub-Milestone 5.3: Integration Testing

**Estimated Effort**: 1 hour
**Priority**: P0 - Blocking
**Status**: Not Started

### Goal

Verify the agent can be created and responds to basic prompts using execute_javascript.

### Implementation

**Manual Testing Script** (for developer verification):

```rust
// Test 1: Agent Creation
let agent = OrchestrationAgent::create(
    "anthropic.claude-3-5-sonnet-20241022-v2:0".to_string(),
    test_credentials,
    "us-east-1".to_string(),
    sender,
    "Test request".to_string(),
    logger,
).await?;

// Verify only 3 tools registered
assert_eq!(agent.tools().len(), 3);

// Test 2: Basic Prompt
let response = agent.run("List all AWS accounts").await?;
// Should contain execute_javascript tool call

// Test 3: Verify Tool Response
// Check that response includes:
// - JavaScript code calling listAccounts()
// - Account data in results
// - User-friendly summary
```

**Integration Test** (optional for MVP):

Create test file `tests/agent_javascript_integration_test.rs`:

```rust
#[tokio::test]
async fn test_orchestration_agent_uses_javascript() {
    // Test that agent uses execute_javascript for account queries
    // Verify prompt includes JavaScript API documentation
    // Verify tool results are presented to user
}
```

### Acceptance Criteria

- [ ] Agent creates successfully with 3 tools
- [ ] Agent prompt includes JavaScript API documentation
- [ ] Manual test shows agent uses execute_javascript for account queries
- [ ] Tool results are properly formatted and presented
- [ ] No errors in agent creation or execution

---

## ✅ Milestone 5 Completion Checklist

### Code Changes
- [ ] Removed 5 tools from orchestration agent (create_task, aws_find_account, aws_find_region, invoke_skill, read_file, list_directory)
- [ ] Kept 3 tools (execute_javascript, todo_write, todo_read)
- [ ] Updated imports to remove unused tool functions
- [ ] Added execute_javascript_tool() helper to mod.rs
- [ ] Updated system prompt to emphasize code-first approach
- [ ] Documented JavaScript APIs in prompt (listAccounts)
- [ ] Documented console functions in prompt
- [ ] Provided code examples and patterns

### Documentation
- [ ] System prompt explains authentication abstraction
- [ ] JavaScript API signatures documented (TypeScript-style)
- [ ] Usage examples provided for each API
- [ ] Return value patterns documented
- [ ] Error handling guidance provided

### Testing
- [ ] Agent creates without errors
- [ ] Only 3 tools registered
- [ ] Prompt includes JavaScript documentation
- [ ] Manual testing shows execute_javascript usage
- [ ] No compilation errors or warnings

---

## Success Criteria

Milestone 5 is complete when:

1. ✅ OrchestrationAgent uses only execute_javascript for infrastructure operations
2. ✅ All unused tools removed and imports cleaned up
3. ✅ Agent prompt emphasizes code-first problem solving
4. ✅ JavaScript APIs documented clearly for LLM consumption
5. ✅ Manual testing shows agent successfully uses JavaScript
6. ✅ No breaking changes to UI or logging infrastructure

---

## Next Steps

After Milestone 5 completion:
- **Milestone 6**: Testing & Validation (CODE_EXECUTION_TOOL_PART_7.md)
  - End-to-end LLM testing with real prompts
  - Performance benchmarking
  - Bug fixes and refinements
  - Add more JavaScript APIs as needed (EC2, S3, Lambda, etc.)

---

## Notes

- **Skills system**: Keep the code but don't use for MVP - may integrate later
- **Task agents**: Remove create_task tool entirely - no more sub-agent spawning
- **TodoWrite/TodoRead**: Keep these as they help with planning (different purpose than execute_javascript)
- **Global credentials**: Keep set_global_aws_credentials() call - still used internally by bindings
- **No breaking UI changes**: Agent manager window and logging continue to work
- **Focus on MVP**: Get basic listAccounts() working end-to-end before adding more APIs

---

## Implementation Order

1. **Sub-Milestone 5.1**: Remove tools and clean up imports (30 min)
2. **Sub-Milestone 5.2**: Update system prompt with JavaScript docs (1 hour)
3. **Sub-Milestone 5.3**: Integration testing and verification (30 min)
4. **Commit**: Commit all changes with comprehensive message
5. **Manual Testing**: Test with real agent UI and user queries

---

## Risks and Mitigations

**Risk**: LLM doesn't understand how to use execute_javascript
- **Mitigation**: Provide clear examples in prompt, test with multiple queries

**Risk**: Breaking existing UI/logging
- **Mitigation**: Only modify tool registration and prompt, keep infrastructure intact

**Risk**: Performance issues with code execution
- **Mitigation**: V8 execution is fast (~5ms), timeout handles long-running code

**Risk**: LLM generates invalid JavaScript
- **Mitigation**: Error messages from V8 are clear, LLM can self-correct

---

**End of Part 6 - Agent Integration Specification**
