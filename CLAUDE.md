# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Stood Library

The `stood` library is the AI agent framework used by the agent_framework module:

- **Repository**: https://github.com/fibanez/stood.git
- **Purpose**: AI agent framework with LLM integration, tools, and conversation management
- **Key components**:
  - Agent implementation and execution loop
  - Conversation and message types (Role, Message, etc.)
  - Tool trait and implementations
  - LLM providers (Bedrock, etc.)
  - Callback system for monitoring agent execution

When debugging agent framework issues, reference the stood repository to understand:
- How `agent.execute()` works
- The conversation message structure and `Role` enum
- How tools are registered and called
- How callbacks work for tracking tool execution and model usage

## CRITICAL: EMOJI/UNICODE RESTRICTIONS IN EGUI

**MOST EMOJIS DO NOT WORK IN EGUI v0.32.3 - USE ASCII TEXT INSTEAD**

This project uses egui v0.32.3 which has LIMITED emoji support:
- Default font: ~1,216 **monochrome** emojis only
- Color emojis: REQUIRE custom fonts (not installed)
- Missing glyph = empty box character

### UNSAFE Characters (DO NOT USE WITHOUT TESTING):
- Circles: ● ○ ◉ ◯ - NOT in default font
- Arrows: → ← ↑ ↓ - Inconsistent rendering
- Checks: ✓ ✗ ✘ - (✔ may work, ✓ doesn't)
- Warning: ⚠ ⚡ - NOT in default font
- Box-drawing: └─ ├─ │ ═ ║ - NOT in default font
- Color emojis: 📁 📂 🗂️ 🗃️ 🗑️ 💾 ✏️ ➕ 📚 📖 🏷️ - REQUIRE custom fonts

### POTENTIALLY SAFE (Monochrome, but verify visually):
- Stars: ★ ☆
- Checkboxes: ☐ ☑
- Checkmark: ✔ (NOT ✓)
- Shapes: ■ ▶
- Arrows: ➡ ⬅ ⬆ ⬇ (basic only)
- Refresh: ↺ ↻
- Heart: ♡

### ALWAYS SAFE - Use These Instead:
- ASCII: `*` `+` `-` `>` `<` `=` `|` `[` `]` `(` `)`
- Words: "Active", "More", "Add", "Edit", "Delete", "Folder"
- Prefixes: "[Active]", "(3 more)", "* Selected"
- Simple indentation with spaces (no box-drawing)

### Examples - CORRECT:
```rust
// Bookmarks
if ui.button("Add Bookmark").clicked() { }
ui.label("* Active Bookmark");  // Asterisk prefix

// Folders
ui.label("Folder: Production");
ui.label("  Bookmark: Web Servers");  // Indentation

// Status
ui.label("OK: Success");
ui.label("WARN: Check config");

// Navigation
ui.label("Account > Region > Resource");
```

### Examples - WRONG (DO NOT DO):
```rust
// WRONG - Emojis that don't render
if ui.button("➕ Add").clicked() { }     // NO - color emoji
ui.label("● Active");                    // NO - not in font
ui.label("📁 Folder");                   // NO - color emoji
ui.label("Account → Region");            // NO - unreliable
ui.label("  ✓ Success");                 // NO - wrong checkmark
ui.label("  ├─ Child");                  // NO - box-drawing
```

**RULE: Default to ASCII text. Only use symbols after visual testing in the actual app.**

## ARCHITECTURAL DECISION AND PROBLEM-SOLVING GUIDELINES

⚠️ **STOP AND COMMUNICATE**: When facing technical obstacles that might lead to compromises, placeholders, or changes from user requirements - STOP and communicate the situation to the user

🚫 **Placeholders are NOT better than broken code** - Mock implementations defeat the purpose and waste time

🎯 **Prioritize Architecture matching user preference over code that compiles** - Don't optimize for compilation success over functional success

🔍 **Solve the real problem** - Research existing patterns, ask for clarification, persist through technical challenges

💬 **Ask for guidance** - Users have experience and can guide architectural decisions; don't make compromises on their behalf

✅ **Only move forward when functionality actually works** - Compilation success without functional success is not progress

**IMPORTANT REMINDERS:**
- ✅ **Test-Driven Development**: Complete and test each feature before moving to the next- create tests in test folder
- ✅ Integration Test: Don't use mock techniques for integration test, all integration test test real behavior using egui_kittest library
- 📚 **Reference Implementation**: Use existing patterns for architectural guidance
- 🔄 **Iterative Progress**: Mark items complete only after successful testing
- 📏 **Token Limits**: Keep implementation chunks manageable for Claude Code sessions
- 📝 **Update Documentation**: Add documentation tasks for new features in this file as you mark coding tasks done - use DOCS_TODO.md for detailed documentation strategy
- 💾 **Git Commits**: ALWAYS use the `/commit` slash command for creating git commits - never use `git commit` directly


## Build/Lint/Test Commands

⚠️ **MEMORY CONSTRAINT WARNING**: This system has many CPUs but limited memory. The test scripts now use full CPU parallelism (28 cores) with memory monitoring to maximize performance while preventing crashes.

### Compilation Caching

**sccache** is configured to accelerate Rust compilations across all working trees:
- **Configuration**: Global Cargo config at `~/.cargo/config.toml` sets `rustc-wrapper = "sccache"`
- **Cache Location**: `/home/fernando/.cache/sccache` (shared across all projects)
- **Cache Size**: 20 GiB limit (increased for aws-dash project complexity and multiple working trees)
- **Check Status**: `sccache --show-stats` to view cache hit/miss statistics
- **Clear Cache**: `sccache --zero-stats` to reset statistics

### Build Commands

- Build: `cargo build`
- Check: `cargo check`
- Web build: `trunk build`
- Lint: `cargo clippy --workspace --all-targets --all-features -- -D warnings -W clippy::all`
- Format: `cargo fmt --all`
- Full check: `./check.sh` (runs all checks in sequence with chunked tests)
- Single test: `cargo test <test_name>` (use test scripts for memory-monitored execution)

### Application Logs

**Main Application Log**:
- Location: `$HOME/.local/share/awsdash/logs/awsdash.log`
- Purpose: General application events, initialization, errors
- Usage: Troubleshoot application startup, configuration issues

**Query Timing Log** (Resource Explorer):
- Location: `$HOME/.local/share/awsdash/logs/query_timing.log`
- Purpose: Detailed timing log for Resource Explorer queries and cache operations
- Content includes:
  - Phase 1 (resource listing) and Phase 2 (enrichment) execution timing
  - Expected queries at start of each phase
  - Cache hits/misses with timing (`[CACHE] GET_HIT`, `[CACHE] GET_MISS`)
  - Cache insertions and evictions (`[CACHE] INSERT`, `[CACHE EVICT]`)
  - Resource query start/end with duration (`[>] START`, `[<] DONE`)
  - Tag fetch operations (`[TAGS] fetch_start`, `[TAGS] fetch_done`)
- Usage: Troubleshoot slow queries, missing resources, spinner stuck issues, cache behavior

**How to Monitor Query Timing**:
```bash
# Tail the query timing log in real-time
tail -f ~/.local/share/awsdash/logs/query_timing.log

# Find phase boundaries
grep -E "PHASE1|PHASE2" ~/.local/share/awsdash/logs/query_timing.log

# Check for cache misses (uncached queries)
grep "GET_MISS" ~/.local/share/awsdash/logs/query_timing.log

# Find slow queries (look for high ms values)
grep "\[<\] DONE" ~/.local/share/awsdash/logs/query_timing.log | sort -t'(' -k2 -n

# Check which resource types completed
grep "\[<\] DONE" ~/.local/share/awsdash/logs/query_timing.log
```

**Per-Agent Logs** (Agent Framework):
- Location: `$HOME/.local/share/awsdash/logs/agents/agent-{uuid}.log`
- Purpose: Detailed per-agent conversation and activity tracking
- Each agent instance creates its own dedicated log file with:
  - Conversation messages (user, assistant, system)
  - Model interactions (requests, responses, token usage)
  - Tool executions (start, success, failure with timing)
  - Sub-task agent creation and progress
  - Agent lifecycle events (creation, rename, termination)
- Usage: Debug agent behavior, review conversations, analyze tool usage
- Find agent log path: Look for "Agent log file:" in agent UI or check logs directory

**Agent Performance Timing Log** (Debug builds only):
- Location: `$HOME/.local/share/awsdash/logs/agent_perf_timing.log`
- Purpose: Detailed timing instrumentation for agent operations
- Only generated in debug builds (zero overhead in release)
- Key metrics tracked:
  - `stood.bedrock.invoke_model` - Model API call duration
  - `stood.bedrock.chat_with_tools` - Full chat cycle with tool handling
  - `stood.event_loop.cycle` - Agent event loop cycle time
  - `MODEL_INVOCATION.agent_execute` - Total agent execution time
  - `awsdash.v8.execute` - JavaScript tool execution time
  - `awsdash.start_task.execute` - Worker task spawn and completion time
- Format: `[timestamp] [thread] operation_name: duration (context)`
- Usage: Analyze model response times, identify bottlenecks, compare model performance

**How to Analyze Agent Performance**:
```bash
# View recent timing entries
tail -100 ~/.local/share/awsdash/logs/agent_perf_timing.log

# Find model invocation times
grep "invoke_model" ~/.local/share/awsdash/logs/agent_perf_timing.log

# Find total agent execution times
grep "MODEL_INVOCATION.agent_execute" ~/.local/share/awsdash/logs/agent_perf_timing.log

# Calculate average invoke_model time
grep "invoke_model" ~/.local/share/awsdash/logs/agent_perf_timing.log | \
  awk -F'=' '{gsub(/ms/,"",$2); sum+=$2; count++} END {print "Average:", sum/count, "ms"}'
```

**How to Monitor Agent Logs**:
```bash
# List all agent logs
ls -lht ~/.local/share/awsdash/logs/agents/

# Tail the most recent agent log
tail -f $(ls -t ~/.local/share/awsdash/logs/agents/*.log | head -1)

# Search for specific patterns across all agent logs
grep -r "invoke_skill" ~/.local/share/awsdash/logs/agents/

# Find logs for a specific agent by name (from log header)
grep -l "Agent Name: MyAgent" ~/.local/share/awsdash/logs/agents/*.log
```

**Stood Debug Traces** (Agent Framework):
- Location: Same as main application log: `$HOME/.local/share/awsdash/logs/awsdash.log`
- Purpose: Comprehensive debug logging for the stood agent framework library
- The tracing subscriber is configured with `stood=trace` to capture all internal operations
- Each trace entry contains:
  - Agent lifecycle events (initialization, execution, completion)
  - Tool execution traces (start, complete, error with full inputs/outputs)
  - Model interactions (requests, responses, streaming)
  - Performance metrics (execution timing, cycle counts)
  - Internal stood library operations for troubleshooting
- Control verbosity via `RUST_LOG` environment variable (e.g., `RUST_LOG=stood=trace`)
- Usage: Troubleshoot orchestration agent issues, analyze execution flow, debug silent failures

**OTLP Telemetry Exports** (OpenTelemetry):
- Endpoint: `http://localhost:4320` (configurable in orchestration_agent.rs)
- Processing mode: Simple processing (real-time export) by default for debugging
- Can be toggled with `STOOD_SIMPLE_TELEMETRY=false` for batch mode
- Telemetry includes: Service attributes, agent type, session IDs, execution spans
- Debug tracing enabled to capture detailed span operations

**Environment Variables for Debug Logging**:
```bash
# Control Rust/stood tracing verbosity (MOST IMPORTANT for debugging)
export RUST_LOG=stood=trace,awsdash=trace  # Full trace-level logging
export RUST_LOG=stood=debug,awsdash=debug  # Debug-level (less verbose)
export RUST_LOG=stood=info,awsdash=info    # Info-level (minimal)

# Use simple (immediate) telemetry export instead of batching
export STOOD_SIMPLE_TELEMETRY=true   # Real-time telemetry (default)
export STOOD_SIMPLE_TELEMETRY=false  # Batch mode (production)
```

**Troubleshooting with Stood Traces**:
```bash
# Tail the main application log (contains both app and stood traces)
tail -f ~/.local/share/awsdash/logs/awsdash.log

# Filter for only stood library traces
tail -f ~/.local/share/awsdash/logs/awsdash.log | grep 'stood::'

# Search for agent execution errors
grep -i 'error' ~/.local/share/awsdash/logs/awsdash.log | grep 'stood'

# Find agent lifecycle events
grep 'Agent.*created\|Agent.*execute\|Agent.*response' ~/.local/share/awsdash/logs/awsdash.log

# Track tool executions
grep 'execute_javascript' ~/.local/share/awsdash/logs/awsdash.log

# Find empty or failed responses
grep 'empty response\|response.*0 chars\|failed' ~/.local/share/awsdash/logs/awsdash.log

# Check model interactions
grep 'model.*request\|model.*response' ~/.local/share/awsdash/logs/awsdash.log

# Monitor in real-time with filtering
tail -f ~/.local/share/awsdash/logs/awsdash.log | grep -E '(stood|Agent|Tool|execute_javascript)'
```

**When to Use Stood Debug Logs**:
- Orchestration agent stops responding with no visible errors
- Tools execute but agent doesn't respond
- Investigating performance issues or timeouts
- Debugging infinite loops or excessive cycles
- Understanding complete tool execution flow
- Analyzing model interactions and token usage

- When creating tests don't create mock tests.  All tests are either unit test, non-mock integration tests, or e2e test with no mocks

### Cleanup Commands

**Agent run cleanup**:
- Clean incomplete agent runs and orphaned workspaces: `./scripts/cleanup-incomplete-runs.sh`
- Removes log files from incomplete runs (older than 24 hours without completion marker)
- Removes orphaned tool workspaces without corresponding completed logs
- Safe to run periodically during development

**Manual cleanup**:
```bash
# Remove all agent logs
rm ~/.local/share/awsdash/logs/agents/*.log

# Remove all tool workspaces
rm -rf ~/.local/share/awsdash/tool_workspaces/*

# View disk usage
du -sh ~/.local/share/awsdash/logs/agents/
du -sh ~/.local/share/awsdash/tool_workspaces/
```

## Chunked Testing Strategy

**For context window management, tests are organized into chunks with clear debugging information:**

- **Fast test suite** (recommended for assistant use): `./scripts/test-chunks.sh fast`
  - Chunk 1: Core tests (frozen, API contracts, unit tests) - ~60 tests, <30s
  - Chunk 2: CloudFormation logic tests - ~50 tests, 1-2min  
  - Chunk 3: UI component tests - ~40 tests, 1-2min
  - Chunk 4: Project management tests - ~25 tests, 30s
  - Documentation tests

- **Complete test suite** (for human use): `./scripts/test-chunks.sh all`
  - Includes all fast chunks plus integration tests (10-30min)

- **Individual chunks**: `./scripts/test-chunks.sh [core|cfn|ui|projects|integration|docs]`

## Smart Verbosity System

**The testing system now supports 4 verbosity levels for optimal context window management:**

### Verbosity Levels

**Level 0 (quiet)**: Minimal output - only test result summaries
```bash
TEST_MODE=quiet ./scripts/test-chunks.sh core
# Output: ✓ test_aws_identity_frozen: test result: ok. 3 passed; 0 failed
```

**Level 1 (smart)** - **DEFAULT**: Perfect for assistants - shows failures without flooding
```bash
./scripts/test-chunks.sh core  # Default mode
# Output: ✓ test_aws_identity_frozen: 3 passed (1s)
#         ❌ ui_basic_test: 4 passed, 1 failed (2s)
#            └─ FAILED test_button_interaction
```

**Level 2 (detailed)**: Shows failure details for debugging
```bash
TEST_MODE=detailed ./scripts/test-chunks.sh core
# Shows failed test names + error excerpts
```

**Level 3 (full)**: Complete output - all cargo test output
```bash
TEST_MODE=full ./scripts/test-chunks.sh core
# Shows every test name and full compilation output
```

### Usage Examples

```bash
# Recommended for assistants (default)
./scripts/test-chunks.sh fast

# Named modes for clarity  
TEST_MODE=smart ./scripts/test-chunks.sh ui
TEST_MODE=detailed ./scripts/test-chunks.sh cfn
TEST_MODE=quiet ./scripts/test-chunks.sh projects

# Numeric levels
TEST_VERBOSITY=1 ./scripts/test-chunks.sh core

# Backwards compatibility
VERBOSE=true ./scripts/test-chunks.sh ui    # Same as Level 3
VERBOSE=false ./scripts/test-chunks.sh ui   # Same as Level 0
```

- **Legacy commands** (use test scripts instead for memory monitoring):
  - All tests: `cargo test --workspace --all-targets --all-features` 
  - Integration tests: `cargo test --test aws_real_world_templates -- --ignored`
  - Integration tests (script): `./scripts/run-integration-tests.sh`

⚠️ **IMPORTANT**: Use the test scripts (`./scripts/test-chunks.sh`, `./scripts/test-with-memory-monitor.sh`) for memory-monitored execution. Direct `cargo test` commands now use full CPU parallelism but may exhaust memory without monitoring.

## Code Style Guidelines

- Follow Rust 2021 edition standards
- Use `#![warn(clippy::all, rust_2018_idioms)]` in all files
- Error handling: Use `anyhow` for error propagation with context
- Logging: Use `log` for basic logging and `tracing` for detailed operation tracking
- Documentation: Use `///` for function/method documentation
- Naming: Use clear, descriptive variable and function names
- Performance: Use caching for expensive operations, profile with `Instant`
- Security: Never log or expose sensitive information
- **UI Text**: Do NOT use emojis in UI text or user-facing strings - egui does not support emoji rendering

## Comment Guidelines: NO Task Tracking References

**NEVER include Phase/Milestone/Task references in comments**

Do NOT use: "Phase 1", "M2.T4", "Task 1", "T1.2", "R3.2", "Sub-Milestone 4.2", "Sprint 3"

```rust
// WRONG
// Phase 2 Batch 1: High-value services
// removed in Phase 1.2
// Property Extraction (M1.T4)
// Will be implemented in M2.T4
#[allow(dead_code)] // Milestone 2
tracing::debug!("R3.2 TESTING - activated");

// CORRECT
// High-value AWS services
// removed
// Property Extraction
// Future enhancement: tag hierarchy
#[allow(dead_code)] // Reserved for future expansion
tracing::debug!("Hint mode activated: {} elements", count);
```

Use instead: "Future enhancement", "removed", "TODO", descriptive names without version numbers

## Keyboard Navigation System

The application implements a Vimium-like keyboard navigation system for efficient keyboard-only operation:

### Global Key Bindings

**Command Palette**:
- **Space Bar** - Opens the command palette (bypasses all Vimium navigation modes)

**Hint Mode** - Visual hints for clicking/focusing elements:
- Type hint letters to filter and activate elements
- `ESC` - Exit hint mode
- `f` - show hints 

### Implementation Notes

- **Space Bar Priority**: The space bar always opens the command palette regardless of current navigation mode
- **Mode Indicators**: Current mode and key sequences are displayed in the navigation status bar
- **Hint Labels**: Uses home row keys (f, j, d, k, s, l, a, ;) for optimal typing ergonomics
- **Focus Integration**: Works seamlessly with the existing window focus management system

### Troubleshooting Hint Mode

**If hint mode shows no hints when pressing `f`:**
1. Check the log file at `$HOME/.local/share/awsdash/logs/awsdash.log` for hint mode debug information
2. Look for log entries like "Enter hint mode with action: Click, 7 elements"
3. Currently using demo elements - real UI element detection is in development
4. Ensure you have windows open (Identity Center, CloudFormation Template, etc.)

## Slash Commands

This project includes custom slash commands in `.claude/commands/` for common workflows:

**Available Commands:**
- `/commit` - Create git commits without Claude/Anthropic references (follows conventional commit style)
- `/dash-docs` - Review changes and update documentation per AWS Dash guidelines
- `/dash-references` - Update reference folder pseudocode documentation
- `/clean-workflow-comments` - Rephrase workflow comments to focus on functionality

**Usage:**
- ALWAYS use `/commit` for creating git commits (never use `git commit` directly)
- Use `/dash-docs` after implementing new features to update technical documentation
- Commands are version-controlled in the repository for team sharing

**Global Commands:**
The following commands are available globally from `~/.claude/commands/`:
- `/squash-merge` - Squash merge current worktree branch into main branch

## Technical Documentation System

This project uses a modular technical documentation system organized in Markdown format for maintainability and cross-referencing:

### Technical Documentation Structure

**Location**: `docs/technical/` - All technical documentation is organized in this directory

**Format**: GitHub Flavored Markdown (`.md` files) with cross-references using `[Display Name](page-name.md)` syntax

**Organization**:
- **`README.md`** - Main documentation index with links to all technical areas
- **System Documentation** - Core system overviews and architecture
- **Implementation Guides** - Step-by-step guides for common development tasks  
- **Reference Documentation** - API references, troubleshooting, and glossaries

### Key Documentation Files

**Core Systems**:
- `window-focus-system.md` - Window focus trait system overview
- `ui-testing-framework.md` - Automated UI testing with egui_kittest

**Development Guides**:
- `adding-new-windows.md` - Complete guide for adding focusable windows
- `ui-component-testing.md` - Writing comprehensive UI tests
- `build-and-test.md` - Development workflow and commands

**Architecture Patterns**:
- `trait-patterns.md` - Common trait implementations and design patterns
- `parameter-patterns.md` - Type-safe parameter passing patterns
- `testing-patterns.md` - Testing strategies and frameworks

### Documentation Guidelines

**For Developers**:
- Always document new systems and patterns in the technical docs
- Use Markdown cross-references to link related concepts
- Include code examples with proper syntax highlighting
- Update `index.md` when adding new documentation areas

**For AI Assistants**:
- Reference technical documentation when implementing similar patterns
- Use existing documentation as templates for new system documentation
- Maintain consistent Markdown formatting and cross-reference structure
- Focus on modular, reusable documentation that can be extended

### Legacy Documentation Files

**When Working on New Features**:
1. Check `docs/technical/README.md` for existing system documentation
2. Reference relevant technical guides for implementation patterns
3. Document new systems in the technical documentation structure
4. Use Markdown format for maintainability and cross-referencing
5. Always run the full test suite to ensure backward compatibility
