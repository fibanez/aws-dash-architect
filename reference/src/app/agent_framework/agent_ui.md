# Agent UI - Reference Documentation

## Component Overview

Renders the agent conversation interface with markdown support for assistant
responses. Provides a chat-style UI with message history, input area, and
action buttons.

---

## Major Functions

| Function | Description |
|----------|-------------|
| `render_agent_chat` | Main UI entry point - renders full chat interface |
| `render_message` | Renders single message (user or assistant) |
| `looks_like_markdown` | Heuristic detection of markdown content |

---

## Implementation Patterns

**Markdown Detection**:
- Pattern: Heuristic string matching
- Algorithm: Linear scan for markdown indicators
- Patterns checked: code blocks, headers, lists, bold, links

**Conditional Rendering**:
- Pattern: Strategy pattern based on content type
- User messages: Plain text with ">" prefix, strong color
- Assistant messages: Markdown if detected, plain text fallback

**Layout Management**:
- Pattern: Height-constrained scroll area + fixed input
- Prevents window auto-growth from long conversations
- Per-agent scroll position via id_salt

---

## External Dependencies

| Dependency | Purpose |
|------------|---------|
| egui | UI framework |
| egui_commonmark | Markdown rendering with CommonMarkViewer |
| syntect | Syntax highlighting for code blocks |

---

## Pseudocode

### render_agent_chat()

```
Input: ui, agent, input_text, markdown_cache
Output: (should_send, log_clicked, clear_clicked, terminate_clicked)

1. Collect agent state (processing, status, messages, id)
2. Calculate max height (available - 150px for controls)

3. Render conversation scroll area:
   - id_salt for per-agent scroll position
   - auto_shrink disabled to prevent collapse
   - max_height capped to prevent growth
   - stick_to_bottom for auto-scroll
   - For each message: render_message(ui, message, cache)

4. Render status line:
   - If processing: show status message or "Processing..."
   - Else: reserve empty space for layout stability

5. Render input area:
   - Multiline TextEdit (3 rows, full width)
   - Enter without Shift sends message
   - Send button (disabled when empty or processing)

6. Render action buttons:
   - Log, Clear Conversation, Terminate Agent

7. Return tuple of button states
```

### looks_like_markdown()

```
Input: content (string)
Output: bool

Patterns to check:
  - "```"      -> code blocks
  - "\n# "     -> H1 header
  - "\n## "    -> H2 header
  - "\n### "   -> H3 header
  - "\n* "     -> unordered list (asterisk)
  - "\n- "     -> unordered list (dash)
  - "\n1. "    -> ordered list
  - "**"       -> bold text
  - "](http"   -> markdown links

Return: any pattern found in content
```

### render_message()

```
Input: ui, message, cache (CommonMarkCache)

Match message.role:
  User:
    - Get strong_text_color from visuals
    - Render with "> " prefix, size 21.0, proportional font

  Assistant:
    - If looks_like_markdown(content):
        CommonMarkViewer::new().show(ui, cache, content)
    - Else:
        ui.label(content)
```

---

## Design Decisions

**Why Heuristic Detection**:
- Avoids parsing overhead for simple responses
- False positives harmless (markdown renders plain text fine)
- Covers common LLM response patterns

**Why CommonMarkCache**:
- Caches parsed markdown for re-rendering efficiency
- Shared across all agents in window
- Passed as mutable reference

**Why Height Constraints**:
- Prevents window from growing with each message
- Maintains stable layout during conversation
- Input area stays fixed at bottom

---

## Related Files

- `src/app/dashui/agent_manager_window.rs` - Creates and passes CommonMarkCache
- `src/app/agent_framework/conversation.rs` - ConversationMessage types
- `docs/technical/agent-framework-v2.md` - Full documentation

---

**Last Updated**: 2025-12-23
