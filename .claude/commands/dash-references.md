---
description: Review git changes and update reference folder pseudocode documentation
---

You are tasked with reviewing code changes from the last day and updating the reference folder to keep pseudocode documentation in sync with the source code.

## Reference Folder Methodology

The `reference/` folder contains pseudocode implementation guides that mirror the `src/` directory structure. Follow these principles from `reference/README.md`:

### File Organization
- **Mirror Source Structure**: Directory structure matches `src/` exactly
- **File Naming**: `source_file.rs` → `source_file.md`
- **Extension**: All reference files use `.md` (Markdown format)

### Content Structure

Each reference file should contain:

1. **Component Overview**: Brief description of the module's purpose
2. **Major Methods/Functions**: List of key functions with single-line descriptions
3. **Implementation Patterns**: Rust idioms, algorithms, design patterns used
4. **External Systems**: Dependencies, AWS services, third-party crates involved
5. **Pseudocode**: Algorithm outlines in 80-column format

### Pseudocode Guidelines

- **Line Length**: Maximum 80 characters per line
- **Format**: Single-line descriptions, concise and clear
- **Style**: Outline-based, hierarchical structure
- **Focus**: How things work, not what they do (implementation vs specification)
- **Rust Patterns**: Reference Rust-specific approaches (traits, lifetimes, async, etc.)
- **Algorithms**: Name specific algorithms (BFS, topological sort, caching, etc.)

### Example Format

```markdown
## Function: parse_template()

Pattern: Builder pattern with error propagation
Algorithm: Recursive descent parser
External: serde_json, serde_yaml

Pseudocode:
  1. Detect format (JSON/YAML) by file extension or content sniffing
  2. Parse raw string into intermediate representation using serde
  3. Validate schema against CloudFormation specification
  4. Build Template struct using builder pattern
  5. Return Result<Template, anyhow::Error> with context
```

### Writing Principles

- **Keep It Concise**: Single-line descriptions, not full documentation
- **Focus on "How"**: Implementation approach, not feature descriptions
- **Use Rust Idioms**: Reference traits, lifetimes, ownership, async patterns
- **80 Columns Max**: Keep pseudocode readable in any editor
- **External Systems**: Always list AWS services, crates, and dependencies

## Instructions

1. **Review recent changes:**
   - Run `git log --since="24 hours ago" --name-only --pretty=format:"%H %s" -- src/`
   - Identify all `.rs` files modified in the last 24 hours
   - Run `git diff HEAD~1 HEAD -- src/` to see the actual changes (or adjust timeframe as needed)

2. **Analyze changed files:**
   - For each modified source file, determine:
     - What functions/methods were added, modified, or removed
     - What algorithms or patterns were introduced or changed
     - What external dependencies were added or updated
     - What Rust patterns are being used (traits, async, lifetimes, etc.)

3. **Update or create reference files:**
   - For each changed `.rs` file in `src/`:
     - Check if corresponding `.md` exists in `reference/src/`
     - If it doesn't exist, create it with full structure
     - If it exists, update the relevant sections:
       - Add new functions to the "Major Methods/Functions" list
       - Update implementation patterns if they changed
       - Add new external dependencies
       - Update or add pseudocode for modified algorithms

4. **Maintain consistency:**
   - Ensure directory structure in `reference/src/` mirrors `src/` exactly
   - Keep pseudocode at 80 columns maximum
   - Use single-line descriptions
   - Focus on implementation approach, not feature specs
   - Always list external crates and AWS services

5. **Quality checklist:**
   - [ ] All modified source files have corresponding reference files
   - [ ] Reference file structure matches `src/` directory structure
   - [ ] All new functions are documented with single-line descriptions
   - [ ] Implementation patterns are clearly identified
   - [ ] External dependencies are listed
   - [ ] Pseudocode follows 80-column limit
   - [ ] Focus is on "how" not "what"
   - [ ] Rust-specific patterns are noted (traits, async, lifetimes, etc.)

## Major Codebase Components (for context)

Reference the following component areas when creating/updating reference files:

- **AWS_IDENTITY**: Identity Center integration, OAuth flow, credential caching, multi-account handling
- **EXPLORER**: Multi-account resource discovery, AWS service clients, data normalization
- **DASHUI**: Window management, keyboard navigation, command palettes, UI components
- **AGENT FRAMEWORK**: Orchestration agents, task agents, tool registry, AWS operations
- **DATA_PLANE**: AWS service API calls, resource management, service operations
- **NOTIFICATIONS**: User notifications, toast messages, alert management

## Git Commands Reference

Useful commands for reviewing changes:

```bash
# Changes in last 24 hours (source files only)
git log --since="24 hours ago" --name-only --pretty=format:"%H %s" -- src/

# Detailed diff for last commit
git diff HEAD~1 HEAD -- src/

# List all changed files in last day
git diff --name-only HEAD@{1.day.ago} HEAD -- src/

# Show changes with context
git log -p --since="24 hours ago" -- src/

# Show file change stats
git diff --stat HEAD@{1.day.ago} HEAD -- src/
```

## Output Format

After reviewing and updating reference documentation, provide:
1. A list of source files modified in the last 24 hours
2. A list of reference files created or updated
3. Summary of key changes documented (new functions, patterns, algorithms)
4. Any notes about missing external dependencies or unclear patterns
