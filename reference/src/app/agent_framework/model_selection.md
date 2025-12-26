# Model Selection - Reference Documentation

## Component Overview

Provides a type-safe enum for selecting Claude and Nova models for agent
creation. Replaces raw model ID strings with compile-time checked variants.

---

## Major Types

| Type | Description |
|------|-------------|
| `AgentModel` | Enum of supported Bedrock models with tool support |

---

## AgentModel Variants

| Variant | Display Name | Notes |
|---------|--------------|-------|
| `ClaudeSonnet45` | Claude Sonnet 4.5 | Default, balanced |
| `ClaudeHaiku45` | Claude Haiku 4.5 | Fast, cost-effective |
| `ClaudeOpus45` | Claude Opus 4.5 | Most capable |
| `NovaPro` | Amazon Nova Pro | AWS native |
| `NovaLite` | Amazon Nova Lite | Lightweight AWS |

---

## Implementation Patterns

**Type-Safe Selection**:
- Pattern: Enum instead of string constants
- Benefit: Compile-time validation of model choices
- Default: ClaudeSonnet45 via #[default] derive

**UI Integration**:
- `display_name()` returns user-friendly labels
- `all_models()` returns static slice for dropdowns
- `Display` trait for string formatting

---

## Trait Implementations

| Trait | Purpose |
|-------|---------|
| `Debug` | Debug formatting |
| `Clone`, `Copy` | Value semantics |
| `PartialEq`, `Eq` | Equality comparison |
| `Default` | Returns ClaudeSonnet45 |
| `Display` | Formats as display_name() |

---

## Pseudocode

### all_models()

```
Output: &'static [AgentModel]

Return static array:
  [ClaudeSonnet45, ClaudeHaiku45, ClaudeOpus45, NovaPro, NovaLite]
```

### display_name()

```
Input: self (AgentModel variant)
Output: &'static str

Match self:
  ClaudeSonnet45 -> "Claude Sonnet 4.5"
  ClaudeHaiku45  -> "Claude Haiku 4.5"
  ClaudeOpus45   -> "Claude Opus 4.5"
  NovaPro        -> "Amazon Nova Pro"
  NovaLite       -> "Amazon Nova Lite"
```

---

## Usage Example

```rust
// In agent creation
let model = AgentModel::default();  // ClaudeSonnet45

// In UI dropdown
for model in AgentModel::all_models() {
    ui.selectable_label(current == *model, model.display_name());
}
```

---

## Design Decisions

**Why Enum Over String**:
- Prevents typos in model IDs
- IDE autocomplete support
- Exhaustive pattern matching
- Version control for model updates

**Why Copy Trait**:
- Enum is small (single byte discriminant)
- Enables pass-by-value semantics
- No heap allocation

---

## Related Files

- `src/app/agent_framework/agent_instance.rs` - Uses AgentModel for agent config
- `src/app/dashui/agent_manager_window.rs` - Model selector dropdown
- `src/app/agent_framework/agent_types.rs` - AgentMetadata includes model

---

**Last Updated**: 2025-12-23
