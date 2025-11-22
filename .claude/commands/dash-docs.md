---
description: Review code changes and update documentation per AWS Dash guidelines
---

You are tasked with reviewing recent code changes and creating or updating documentation according to AWS Dash's documentation guidelines.

## Documentation Philosophy

Follow the AWS Dash documentation principles from `TODOS/DOCS_TODO.md`:

### Core Principles
- **Progressive Disclosure**: Start with "How to Use" → progress to "How it Works" → finally "How to Extend"
- **Multi-Audience Support**: Serve both application users and developers with distinct pathways
- **Unified Documentation**: In-source rustdoc comments + Markdown extended docs
- **Continuous Validation**: Every commit triggers doc review
- **Cognitive Load Management**: Consistent structure reduces mental overhead
- **User-Focused Writing**: Documentation that speaks directly to users' needs and goals

### Writing Style Guidelines

**Voice and Tone:**
- User-obsessed: Frame benefits from the user's perspective ("You'll be able to..." rather than "The library provides...")
- Confident but not boastful: State features and benefits matter-of-factly
- Conversational yet professional: Use accessible language without being overly casual
- Action-oriented: Emphasize what users can do or achieve
- Trustworthy: Provide clear, straightforward communication without marketing fluff

**Writing Principles:**
- Clarity above all: Choose simple words over complex ones, use short sentences when possible
- Scannable format: Use bullet points for features, bold text for key information
- Front-loaded benefits: Place the most important information first
- Specific over vague: "Processes 10,000 records per second" not "high performance"
- Active voice: "Parse the configuration" not "The configuration can be parsed"

**Markdown Formatting:**
- Standard emphasis: Use `**text**` for bold, `*text*` for italic
- Consistent formatting: Follow GitHub Flavored Markdown standards
- Cross-reference format: Use `[Display Text](file-name.md)` for internal links
- Source code links: Use `[Link Description](../src/module/mod.rs)` to link Markdown to source files
- Code blocks: Use triple backticks with language specification: ```rust

**What to Avoid:**
- Exclamation points (!)
- Superlatives and hyperbole ("amazing," "incredible," "best ever")
- Industry jargon without explanation
- Long paragraphs in API descriptions
- Passive voice constructions
- Marketing language or sales pitch tone
- Assumptions about user's knowledge level without context

## Instructions

1. **Review the changes:**
   - Run `git status` to see untracked files
   - Run `git diff` to see staged and unstaged changes
   - Identify which systems, components, or features were modified

2. **Identify documentation needs:**
   - Check if new systems/patterns need documentation in `docs/technical/`
   - Determine if existing docs need updates
   - Verify if rustdoc comments need to be added/updated
   - Check if `docs/technical/README.md` needs updating

3. **Create or update documentation:**
   - For new systems: Create new `.md` files in `docs/technical/`
   - For existing systems: Update relevant documentation files
   - Add rustdoc comments (`///`) to new/modified public functions and structs
   - Use the writing style guidelines above
   - Include code examples with proper syntax highlighting
   - Add cross-references using `[Display Text](file-name.md)` format

4. **Update the documentation index:**
   - Add new documentation files to `docs/technical/README.md`
   - Ensure proper categorization (Core Systems, Development Guides, Architecture Patterns, etc.)
   - Maintain alphabetical or logical ordering

5. **Quality checklist:**
   - [ ] Every major internal component has rustdoc documentation
   - [ ] Every test has explanatory comment
   - [ ] Every unsafe block has safety documentation
   - [ ] Error handling patterns are documented
   - [ ] System interaction patterns are documented
   - [ ] Cross-references are valid
   - [ ] No outdated information remains
   - [ ] Code examples are included where helpful
   - [ ] Documentation follows the writing style guidelines

## Example Documentation Structure

**For a new system (e.g., `docs/technical/my-new-system.md`):**

```markdown
# My New System

Brief one-sentence description of what the system does.

## Overview

What the system is and why it exists. Focus on user benefits.

## How to Use

Step-by-step guide for using the system. Include code examples.

## How it Works

Technical details about the implementation.

## Integration Points

How this system connects with other parts of the application.

## Testing

How to test code that uses this system.

## Related Documentation

- [Related System](related-system.md)
- [Source Code](../src/module/mod.rs)
```

## Output Format

After reviewing and updating documentation, provide:
1. A summary of what was changed in the code
2. A list of documentation files created or updated
3. Key documentation additions or changes made
4. Any additional documentation work needed (if applicable)
