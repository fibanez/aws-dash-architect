---
description: Validate documentation references against actual codebase
---

You are tasked with validating that documentation accurately references existing code. This is the reverse of `/dash-docs` - it checks docs → code instead of code → docs.

## Validation Goals

Find documentation that references code that no longer exists:
- File paths that don't exist
- Functions/structs/modules that were removed or renamed
- Outdated code examples
- Broken cross-references between documentation files

## Instructions

1. **Gather documentation files:**
   - List all `.md` files in `docs/technical/`
   - Include `CLAUDE.md` and any other root-level documentation

2. **Extract code references from each doc:**
   For each documentation file, identify:
   - **File path references**: Patterns like `src/module/file.rs`, `src/app/component/mod.rs`
   - **Source code links**: Markdown links like `[text](../src/path/file.rs)`
   - **Code block references**: Function/struct names in code examples
   - **Key Files sections**: Lists of important source files
   - **Module references**: References to Rust modules like `normalizers/json_expansion.rs`

3. **Validate each reference:**
   - Check if referenced files exist using Glob or file reads
   - For struct/function references in code examples, verify they exist in the codebase
   - Check cross-reference links between `.md` files

4. **Categorize findings:**

   **Critical (code doesn't exist):**
   - Referenced source files that don't exist
   - Documented modules that were deleted
   - Broken links to source code

   **Warning (possibly stale):**
   - Code examples with function signatures that don't match current code
   - References to renamed structs/functions
   - Cross-references to deleted documentation files

   **Info (review recommended):**
   - Documentation for deprecated features
   - Very old documentation that may need refresh

5. **Check for orphaned documentation:**
   - Documentation files not linked from `docs/technical/README.md`
   - Documentation for systems that no longer exist

## Output Format

Provide a structured report:

```
## Documentation Validation Report

### Critical Issues (broken references)
- `docs/technical/foo.md`: References `src/app/bar.rs` - FILE NOT FOUND
- `docs/technical/baz.md`: Links to `old-system.md` - DOC NOT FOUND

### Warnings (possibly stale)
- `docs/technical/api.md`: Code example shows `fn old_name()` but code has `fn new_name()`
- `docs/technical/config.md`: Describes deprecated `ConfigV1` struct

### Orphaned Documentation
- `docs/technical/unused-guide.md`: Not linked from README.md

### Valid Documentation
- `docs/technical/resource-explorer-system.md`: All 12 references valid
- `docs/technical/resource-normalizers.md`: All 8 references valid

### Summary
- Files checked: X
- References validated: Y
- Critical issues: Z
- Warnings: W
```

## Validation Patterns

**File path patterns to check:**
```
src/.../*.rs
src/app/...
normalizers/*.rs
aws_services/*.rs
```

**Markdown link patterns:**
```markdown
[Display Text](../src/path/file.rs)
[Doc Link](other-doc.md)
`src/module/file.rs`
```

**Key Files section pattern:**
```markdown
**Key Files:**
- `src/app/module/file.rs` - Description
```

## Do NOT

- Modify any files (this is read-only validation)
- Suggest fixes in this command (that's what `/dash-docs` is for)
- Report valid references as issues
- Flag intentionally abstract examples (like `newservice.rs` templates)

## After Validation

If issues are found, inform the user they can:
1. Run `/dash-docs` to update documentation based on current code
2. Manually fix specific broken references
3. Delete orphaned documentation files
