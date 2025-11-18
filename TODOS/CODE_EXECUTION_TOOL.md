# CODE EXECUTION TOOL IMPLEMENTATION

**Created**: 2025-11-13
**Status**: Planning
**Priority**: High

---

## Overview

Implement a code execution tool that allows AI agents to run JavaScript/TypeScript
code in isolated V8 sandboxes. The LLM generates code and executes it via a tool
call, enabling programmatic data processing, calculations, and logic that would
be difficult with natural language alone.

---

## Core Concept

- **V8 Isolates**: Each code execution runs in a lightweight, isolated V8 environment
- **LLM-Generated Code**: Agent writes JS/TS code and calls the execution tool
- **Sandboxed Execution**: Security boundaries prevent malicious or accidental damage
- **Result Integration**: Execution results flow back to the agent for further processing

---

## Architecture Decisions

### ✅ DECISION 1: Simplified Agent Architecture
- **Remove all existing tools** from the orchestration agent
- **Remove task agent system** entirely
- **Single agent**: Orchestration agent only
- **Single tool**: Code execution tool only
- The agent solves ALL problems by writing and executing JavaScript code

### ✅ DECISION 2: Rust-Bound Functions (NOT Node.js APIs)
- **No Node.js built-ins** (no fs, http, crypto, etc.)
- **No AWS SDK in JavaScript**
- **Rust functions bound to V8 isolate** using rusty_v8
- The LLM gets function signatures/descriptions in the prompt
- The LLM writes JavaScript that calls ONLY these bound Rust functions
- All I/O, AWS operations, file access happens through Rust bindings

**Example Flow**:
```
1. Rust: Register function `aws_list_ec2_instances(region: string) -> Array<Instance>`
2. Prompt: Document function signature for LLM
3. LLM writes: `const instances = aws_list_ec2_instances('us-east-1');`
4. V8 isolate: Calls bound Rust function
5. Rust: Makes AWS SDK call, returns results to JavaScript
6. JavaScript: Processes results, returns to agent
```

### ✅ DECISION 3: Use rusty_v8 for Bindings
- Use `rusty_v8` crate for V8 integration
- Reference rusty_v8 test examples for function registration patterns
- Security enforced at Rust level, not JavaScript sandbox level

### ✅ DECISION 4: Module Organization
- Create `src/app/agent_framework/v8_bindings/` module
- Organize bound functions by domain (aws/, files/, utilities/)
- Central registration point for all functions

### ✅ DECISION 5: Static Function Registration (MVP)
- **Static registration**: Register ALL available functions at isolate creation
- **Why**: Much simpler than dynamic (~20 lines vs ~200+ lines)
- **Trade-off**: Negligible memory overhead for unused functions
- **Future**: Can optimize to dynamic if we reach hundreds of functions

### ✅ DECISION 6: MVP Scope - Single Function
- **First function**: `listAccounts()`
- **Implementation**: Reads AWS account data from application configuration
- **Abstraction**: Hides AWS-specific details (SSO, regions, credentials) from LLM
- **Purpose**: Proves the binding mechanism works
- **Simplicity**: Simple enough to validate the entire architecture

### ✅ DECISION 7: Function Documentation Format & Prompt Strategy

**CRITICAL**: The bound functions **abstract away all AWS complexity**. The LLM prompt must make clear:
- **NO credentials needed** - authentication handled internally
- **NO region selection needed** - configuration handled internally
- **NO AWS-specific setup** - just call the provided functions
- **Problem-solving focus** - LLM writes logic using simple function APIs

Provide comprehensive documentation in agent prompt combining:
1. **TypeScript-style signature**: `function listAccounts(): Account[]`
2. **JSDoc comments**: Purpose, parameters, return value, **no AWS specifics**
3. **Usage examples**: Actual JavaScript code snippets
4. **Type definitions**: Interface definitions for complex return types
5. **Prompt framing**: "You have access to these APIs" (not "AWS APIs")

**Example Documentation Format**:
```typescript
/**
 * List all configured accounts
 *
 * Returns an array of account objects available in the system.
 * No credentials or configuration needed - this is handled internally.
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
 */
function listAccounts(): Account[];

interface Account {
  id: string;          // Unique account identifier
  name: string;        // Human-readable name
  // Internal fields handled automatically, no need to specify
}
```

**Prompt Introduction Example**:
```
You are an agent that solves problems by writing JavaScript code. You have access to
a set of APIs that allow you to interact with the infrastructure.

IMPORTANT:
- Authentication and credentials are handled internally - don't ask for them
- Configuration (regions, etc.) is managed automatically - don't ask for it
- Focus on solving the user's problem using the provided function APIs

Available APIs:
[Function documentation here]
```

### ✅ DECISION 8: Tool Name
- **Tool name**: `execute_javascript`
- Clear, explicit name that indicates what the tool does
- LLM will understand this is for running JavaScript code

### ✅ DECISION 9: Tool Input Format
```rust
{
  "code": "const accounts = listAccounts();\nconsole.log(accounts);\nreturn accounts;"
}
```
- **Single parameter**: Just the JavaScript code as a string
- **No separate input data**: Filters/parameters should be in the JavaScript code itself
- **LLM responsibility**: Write complete, self-contained JavaScript programs
- **Abstraction**: Functions handle all infrastructure details internally

### ✅ DECISION 10: Tool Output Format
```rust
// Success case
{
  "result": <JSON value>,           // Return value from the code
  "stdout": "console output...",     // All console.log() calls
  "stderr": "",                      // All console.error() + exceptions
  "execution_time_ms": 45,           // Execution duration
  "success": true                    // Overall execution status
}

// Runtime error case
{
  "result": null,
  "stdout": "partial output...",
  "stderr": "TypeError: Cannot read property 'id' of undefined\n    at <anonymous>:3:15",
  "execution_time_ms": 12,
  "success": false
}

// Compilation error case
{
  "result": null,
  "stdout": "",
  "stderr": "SyntaxError: Unexpected token '}' at line 5",
  "execution_time_ms": 0,
  "success": false
}
```

### ✅ DECISION 11: Error Handling Strategy
- **Syntax errors**: Caught during V8 compilation (before execution) → format as stderr
- **Runtime exceptions**: TypeError, ReferenceError, etc. → captured with stack trace → stderr
- **console.error()**: Explicit error logging → stderr
- **Uncaught promises**: Async errors → stderr
- **success boolean**: Quick check for LLM to determine if retry needed
- **All errors flow through stderr**: Unified error channel

### ✅ DECISION 12: Security & Resource Limits

**Execution Timeout**:
- **Configurable**: Can be set per agent or globally
- **Default**: 30 seconds
- **Enforcement**: V8 isolate will be terminated after timeout
- **Infinite loops**: Handled by timeout mechanism

**Memory Limit**:
- **256MB heap size** per isolate
- Set via V8 isolate creation parameters
- Prevents memory exhaustion attacks

**Concurrency Model**:
- **One isolate per agent** (not shared between agents)
- **Multiple agents supported** (each gets its own isolate)
- **Simple, safe**: No isolate pooling complexity in MVP
- **Agent isolation**: Each agent's code executions are independent

**Isolate Lifecycle**:
- **Create new isolate for each execution**: Fresh state every time
- **Startup cost**: ~100ms (acceptable for MVP)
- **No state persistence**: Variables/closures don't leak between executions
- **Simpler reasoning**: Each execution is completely independent

**V8 Binary Management**:
- **Platform-specific binary**: rusty_v8 provides pre-built V8 for each platform
- **Cache location**: `~/.local/share/awsdash/v8/` (or platform equivalent)
- **Download once**: Binary cached on first use, reused thereafter
- **Version management**: Track V8 version for compatibility

### ✅ DECISION 13: JavaScript Execution Mechanics

**Return Value Mechanism**:
- **Prompt-driven**: Tell LLM in prompt how to return values
- **Flexible**: Can be last expression, explicit return, or any pattern
- **Example instruction**: "Use explicit `return` statement at the end of your code"
- **V8 captures**: Whatever the script evaluates to becomes the result

**Language Support**:
- **JavaScript only** for MVP
- **No TypeScript**: Avoids compilation complexity
- **Future**: Could add TS support later if needed

**Console Implementation**:
- **console.log()** → stdout string
- **console.error()** → stderr string
- **console.warn()** → stdout string (treated as info)
- **console.debug()** → stdout string (treated as info)
- Bound as Rust functions that append to output buffers

**Async/Promise Support**:
- **Synchronous only** for MVP
- All bound Rust functions return immediately (no Promises)
- `listAccounts()` returns `Account[]` directly, not `Promise<Account[]>`
- **Simpler implementation**: No need for async runtime in V8
- **Future**: Can add async support if needed

### ✅ DECISION 14: Integration with Existing Agent System

**Agent System Changes**:
- **Remove all existing tools** from OrchestrationAgent:
  - aws_find_account ❌
  - aws_find_region ❌
  - create_task ❌
  - All AWS operation tools ❌
- **Remove task agent system entirely**: No more sub-agent spawning
- **Keep**: Agent logging infrastructure, UI components, agent instance management
- **Single tool**: Only `execute_javascript` remains

**Configuration & Initialization**:
- **Automatic registration**: `execute_javascript` tool registered with all agents by default
- **V8 initialization**: At application startup (not lazy)
  - Initialize V8 platform
  - Download/cache V8 binary if needed
  - Register global bindings
- **Fail fast**: If V8 fails to initialize, application should not start

**Skills System**:
- **Keep the code**: Don't delete the skills system implementation
- **Don't use for now**: Won't be active in MVP
- **Future**: Will return to integrate skills with code execution
- **Reason**: May want LLM to load skills as JavaScript libraries later

**Infrastructure Operations via Bound Functions**:
- All infrastructure operations exposed through simple JavaScript APIs
- **Abstraction**: Functions hide AWS complexity (credentials, regions, SDKs)
- Start with `listAccounts()`, then expand to resources, services, etc.
- Reuse existing AWS SDK client logic internally (transparent to LLM)

**Testing Strategy**:
1. **Phase 1 - V8 Binding Validation**:
   - Recreate rusty_v8 test examples in our codebase
   - Test function binding mechanism works
   - Test console.log/error capture
   - Test return value extraction
   - Test error handling (syntax, runtime)

2. **Phase 2 - End-to-End LLM Testing**:
   - Write prompts that instruct LLM to use execute_javascript
   - Test LLM writes valid JavaScript
   - Test LLM calls bound functions correctly
   - Test LLM handles errors and retries
   - Manual validation with real agent interactions

---

## Implementation Notes

**Multi-Part Documentation**: This implementation will be broken into multiple
parts (CODE_EXECUTION_TOOL_PART_2.md, PART_3.md, etc.) to keep context
manageable during development. Each part will focus on specific milestones.

---

## Technical Requirements

### Rust Dependencies
- **rusty_v8**: V8 JavaScript engine bindings for Rust
- **serde_json**: JSON serialization for data exchange between Rust ↔ JavaScript
- **tokio**: Async runtime for timeout handling
- **anyhow**: Error handling

### Module Structure
```
src/app/agent_framework/
├── v8_bindings/
│   ├── mod.rs                    # Module exports
│   ├── runtime.rs                # V8 isolate creation, execution, lifecycle
│   ├── console.rs                # console.log/error/warn/debug implementations
│   ├── bindings/
│   │   ├── mod.rs                # Binding registry
│   │   ├── aws_accounts.rs       # listAwsAccounts() implementation
│   │   └── [future bindings]     # Additional bound functions
│   └── types.rs                  # Shared types for Rust ↔ JS conversion
├── tools/
│   └── execute_javascript.rs     # Tool implementation
```

### Key Components

**1. V8Runtime** (runtime.rs):
- Initialize V8 platform and isolate
- Set memory limits (256MB)
- Execute JavaScript code with timeout (30s default)
- Capture stdout/stderr
- Extract return values
- Handle errors (syntax, runtime, timeout)

**2. ConsoleFunctions** (console.rs):
- Bind console.log() → append to stdout buffer
- Bind console.error() → append to stderr buffer
- Bind console.warn() → append to stdout buffer
- Bind console.debug() → append to stdout buffer

**3. BindingRegistry** (bindings/mod.rs):
- Central registration point for all bound functions
- Called during isolate creation
- Extensible for adding new functions
- **Abstraction layer**: Maps simple JS function names to complex Rust/AWS operations

**4. ExecuteJavaScriptTool** (tools/execute_javascript.rs):
- Implements Tool trait
- Creates fresh V8Runtime per execution
- Returns structured result (success, result, stdout, stderr, time)

### Data Flow
```
1. Agent calls execute_javascript("const accounts = listAccounts(); return accounts;")
2. ExecuteJavaScriptTool creates new V8Runtime
3. V8Runtime initializes isolate with 256MB limit, 30s timeout
4. BindingRegistry registers all functions (listAccounts, console.*, etc.)
5. V8Runtime executes JavaScript code
6. JavaScript calls listAccounts() → Rust function invoked
7. Rust function:
   - Uses internal AWS SDK client (credentials already configured)
   - Reads account data from application state
   - Returns JSON to JavaScript (abstracts AWS details)
8. JavaScript processes data, returns result
9. V8Runtime captures return value + stdout/stderr
10. ExecuteJavaScriptTool formats response, returns to agent
```

---

## Milestones Overview

### Milestone 1: V8 Infrastructure Setup (~2-3 days)

**📄 Detailed Spec**: [CODE_EXECUTION_TOOL_PART_2.md](./CODE_EXECUTION_TOOL_PART_2.md)

**Summary**:
- Add rusty_v8 dependency with **automatic binary download** (pre-compiled .a file)
  - **Fast**: ~5 minutes download, NOT 30-60 minutes compilation
  - **Pre-compiled**: V8 C++ engine already compiled, ready to link
  - **Default behavior**: cargo build downloads from GitHub releases
- Initialize V8 platform at app startup (global singleton)
- Create V8Runtime for isolate management and execution
- Implement timeout mechanism (30s default, configurable)
- Enforce memory limits (256MB default, configurable)
- Create cleanup script for testing from scratch
- Comprehensive test suite (basic, timeout, memory, integration)

**Sub-Milestones**:
1. Add rusty_v8 dependency & binary cache setup
2. Global V8 platform initialization
3. V8Runtime - isolate creation & basic execution
4. Timeout mechanism with IsolateHandle
5. Memory limit enforcement
6. Integration & documentation

**Deliverables**:
- `src/app/agent_framework/v8_bindings/platform.rs` - Global platform management
- `src/app/agent_framework/v8_bindings/runtime.rs` - Execution runtime
- `scripts/clean-v8.sh` - Cache cleanup script
- `tests/v8_*_test.rs` - Test suites
- `scripts/verify-milestone-1.sh` - Verification script

**TDD Approach**: Write failing tests → implement → verify passing

---

### Milestone 2: Console Binding (~1 day)

**📄 Detailed Spec**: CODE_EXECUTION_TOOL_PART_3.md *(to be created)*

**Summary**:
- Implement console.log/error/warn/debug as Rust-bound functions
- Capture output to stdout/stderr buffers
- Integrate with V8Runtime execution flow
- Unit tests for console output capture

**Deliverables**:
- `src/app/agent_framework/v8_bindings/console.rs` - Console bindings
- Tests for output capture

---

### Milestone 3: Function Binding System (~2 days)

**📄 Detailed Spec**: CODE_EXECUTION_TOOL_PART_4.md *(to be created)*

**Summary**:
- Implement BindingRegistry pattern for function registration
- Create first bound function: `listAccounts()` (abstracts AWS account reading)
- Implement Rust ↔ JavaScript type conversion (JSON)
- Unit tests for function binding and invocation
- **Focus**: Clean abstraction that hides infrastructure complexity from LLM

**Deliverables**:
- `src/app/agent_framework/v8_bindings/bindings/mod.rs` - Registry
- `src/app/agent_framework/v8_bindings/bindings/accounts.rs` - First binding
- `src/app/agent_framework/v8_bindings/types.rs` - Type conversion utilities

---

### Milestone 4: Tool Implementation (~1 day)

**📄 Detailed Spec**: CODE_EXECUTION_TOOL_PART_5.md *(to be created)*

**Summary**:
- Create ExecuteJavaScriptTool implementing Tool trait
- Integrate with agent tool system
- Return structured result format (ExecutionResult)
- Error handling (syntax, runtime, timeout)

**Deliverables**:
- `src/app/agent_framework/tools/execute_javascript.rs` - Tool implementation
- Integration tests with tool system

---

### Milestone 5: Agent Integration (~1-2 days)

**📄 Detailed Spec**: CODE_EXECUTION_TOOL_PART_6.md *(to be created)*

**Summary**:
- Remove all existing tools from OrchestrationAgent
- Remove task agent system
- Register execute_javascript as sole tool
- Update agent prompt with JavaScript execution instructions
- Document bound functions in prompt (TypeScript-style with examples)

**Deliverables**:
- Updated `src/app/agent_framework/agents/orchestration_agent.rs`
- New agent prompt with JavaScript documentation
- Tool registration cleanup

---

### Milestone 6: Testing & Validation (~2-3 days)

**📄 Detailed Spec**: CODE_EXECUTION_TOOL_PART_7.md *(to be created)*

**Summary**:
- **Phase 1**: V8 binding tests (recreate rusty_v8 examples)
- **Phase 2**: End-to-end LLM testing with prompts
- Manual validation with real agent interactions
- Bug fixes and refinements

**Deliverables**:
- Comprehensive test suite
- Integration tests with LLM
- Performance benchmarks
- Documentation updates

---

**Total Estimated Time: 9-12 days**

**Development Approach**: Test-Driven Development (TDD)
- Write failing tests first
- Implement functionality
- Verify tests pass
- Refactor and iterate

**Documentation Structure**: Each milestone has detailed specifications in separate part files to keep context manageable during implementation.

---

## References

- V8 isolate documentation: https://v8.dev/docs
- rusty_v8 crate: https://crates.io/crates/rusty_v8
- rusty_v8 examples: https://github.com/denoland/rusty_v8/tree/main/examples
- Deno/Bun security models (inspiration for sandboxing patterns)

---

## Summary

This document captures the complete architectural plan for implementing JavaScript code execution in the AWS Dashboard agent framework.

**Key Decisions**:
- Single agent (OrchestrationAgent) + single tool (execute_javascript)
- All capabilities via Rust-bound functions, not Node.js APIs
- V8 isolates with 256MB memory limit and 30s timeout
- Static function registration for simplicity
- MVP: Synchronous JavaScript only, one bound function (listAwsAccounts)

**Implementation Approach**:
1. Start with V8 infrastructure and basic execution
2. Add console bindings for output capture
3. Implement function binding system with first AWS function
4. Create tool and integrate with agent
5. Test thoroughly (unit tests → LLM integration tests)

**Estimated Timeline**: 9-12 days for complete MVP

**Next Steps**:
- Begin Milestone 1 (V8 Infrastructure Setup)
- Create Part 2 document with detailed implementation tasks

---

**End of Part 1 - Planning & Requirements**
