# Skills Module - Agent Skill Discovery and Loading

## Component Overview

Provides skill discovery and loading for extending agent capabilities.
Skills are markdown files with YAML frontmatter defining reusable behaviors.

**Pattern**: Plugin discovery with lazy loading
**External**: walkdir, serde_yaml
**Purpose**: Extensible agent capabilities

---

## Module Structure

- `mod.rs` - Module exports
- `discovery.rs` - SkillDiscoveryService, SkillMetadata
- `loader.rs` - SkillLoader, LoadedSkill
- `manager.rs` - SkillManager global singleton

---

## Key Types

### SkillMetadata
Skill metadata from YAML frontmatter:
- `name`: Skill identifier
- `description`: Human-readable description
- `triggers`: Keywords that activate skill
- `version`: Skill version

### LoadedSkill
Fully loaded skill content:
- `metadata`: SkillMetadata
- `content`: Full markdown content
- `path`: File system path

### SkillManager
Global singleton for skill access:
- Lazy discovery on first access
- Caches discovered skills
- Provides lookup by name/trigger

---

## Skill Discovery Paths

Skills are discovered from:
1. `~/.claude/skills/` - User skills
2. `~/.awsdash/skills/` - Application skills

### Skill File Format

```markdown
---
name: analyze-costs
description: Analyze AWS cost patterns
triggers:
  - cost
  - spending
  - billing
version: "1.0"
---

# Cost Analysis Skill

Instructions for analyzing AWS costs...
```

---

## Usage

```rust
let manager = get_skill_manager();
if let Some(skill) = manager.find_by_trigger("cost") {
    let loaded = manager.load_skill(&skill.name)?;
    // Use loaded.content in agent prompt
}
```

---

**Last Updated**: 2025-12-22
**Status**: Accurately reflects skills/ module structure
