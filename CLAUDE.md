# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

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
- The application log files is located in $HOME/.local/share/awsdash/logs/awsdash.log - this file can be used to troubleshoot or debug errors
- When creating tests don't create mock tests.  All tests are either unit test, non-mock integration tests, or e2e test with no mocks

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

## Graph Visualization Guidelines

- **NO GRID BACKGROUND** in CloudFormation Scene Graph window - clean background only

## Custom File Selection

The application uses a custom fuzzy-search file picker instead of native file dialogs:
- The current directory's contents are displayed with folders first
- Type to filter items with fuzzy matching
- Press Ctrl-Y to select a folder and navigate into it
- Press Left Arrow (←) to go up one level in the directory
- Press Enter to accept the current path
- Press Esc to cancel selection
- Press Ctrl+N to create a new folder

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
- `cloudformation-system.md` - Template parsing and visualization
- `cloudformation-manager.md` - Comprehensive CloudFormation template management system
- `project-management.md` - Project structure and resource tracking

**Development Guides**:
- `adding-new-windows.md` - Complete guide for adding focusable windows
- `cloudformation-manager-development.md` - Extending CloudFormation Manager functionality
- `cloudformation-manager-testing.md` - CloudFormation Manager testing strategies
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

The following files contain historical implementation tracking:
- **TODOS/IMPLEMENTATION_COMPLETE.md** - Completed work archive (legacy)

**When Working on New Features**:
1. Check `docs/technical/README.md` for existing system documentation
2. Reference relevant technical guides for implementation patterns
3. Document new systems in the technical documentation structure
4. Use Markdown format for maintainability and cross-referencing
5. Always run the full test suite to ensure backward compatibility
