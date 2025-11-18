# Message System - Agent-to-UI Event Communication

## Component Overview

Defines message types for agent-to-UI communication, including streaming
content, tool execution events, JSON debugging, and completion notifications.
Provides tree-structured message representation for hierarchical display.

**Pattern**: Enum-based event system with nested message structures
**External**: serde for serialization, chrono for timestamps
**Purpose**: Decouple agent execution from UI rendering

---

## Major Types

### AgentResponse (Event Enum)
- `Success(AgentResult)` - Agent execution completed successfully
- `Error(String)` - Agent error or failure
- `JsonDebug(JsonDebugData)` - Model request/response JSON capture
- `ModelChanged { model_id: String }` - Model ID updated
- `ToolCallStart { parent_message }` - Tool execution begins
- `ToolCallComplete { parent_message_id, child_message }` - Tool finishes

### Message (Tree Node)
- `id: String` - Unique message identifier
- `role: MessageRole` - User/Assistant/System/Debug/JsonRequest/JsonResponse
- `content: String` - Message text content
- `timestamp: DateTime<Utc>` - Message creation time
- `summary: Option<String>` - Collapsed view label
- `nested_messages: Vec<Message>` - Child messages (for tools)
- `agent_source: Option<String>` - Agent that created message
- `json_debug_data: Vec<JsonDebugData>` - Attached JSON debugging

### JsonDebugData
- `json_type: JsonDebugType` - Request vs Response
- `json_content: String` - Pretty-printed JSON
- `raw_json_content: Option<String>` - Raw model JSON (if available)
- `timestamp: DateTime<Utc>` - Capture time

---

## Implementation Patterns

### Pattern: Message Tree Construction

**Algorithm**: Parent-child linking via message IDs
**External**: HashMap for O(1) lookup during tree building

Pseudocode:
  1. Agent sends ToolCallStart with parent_message
  2. UI stores message in flat list, indexes by message.id
  3. Agent sends ToolCallComplete with parent_id + child_message
  4. UI finds parent by parent_id in index
  5. Append child to parent.nested_messages Vec
  6. Render tree recursively: parent → children → grandchildren
  7. Summary field used for collapsed tree nodes

### Pattern: Streaming Content Aggregation

**Algorithm**: Incremental string concatenation with chunked updates
**External**: String::push_str, UI re-render on each chunk

Pseudocode:
  1. Agent streams StreamContent("Hello") event
  2. UI appends to current message content
  3. Agent streams StreamContent(" world") event
  4. UI appends to same message
  5. Final message content: "Hello world"
  6. Role: MessageRole::Assistant for LLM output
  7. Timestamp: first chunk determines message timestamp

### Pattern: JSON Debug Capture

**Algorithm**: Dual capture (pretty + raw) with type tagging
**External**: serde_json::to_string_pretty

Pseudocode:
  1. Callback handler intercepts ModelStart event
  2. Serialize request to pretty JSON string
  3. Create JsonDebugData with type=Request, timestamp=now
  4. Send AgentResponse::JsonDebug to UI
  5. UI stores in separate debug panel
  6. On ModelComplete: repeat for response JSON
  7. UI displays side-by-side request/response view

---

## External Dependencies

- **serde**: Serialization for JSON debugging
- **chrono**: UTC timestamps for all messages
- **std::fmt**: Display trait for message roles

---

## Key Algorithms

### Message ID Generation
- Uses Uuid::new_v4().to_string() for uniqueness
- Ensures uniqueness for parent-child linking
- UUID-based for collision-free identification

### Tree Traversal for Rendering
- Depth-first: parent → nested → next sibling
- Indentation level tracks tree depth
- Summary field enables collapsible sections

### Role-Based Styling
- MessageRole enum maps to UI colors/icons via color() and icon() methods
- Dark mode vs light mode color variants
- User: "👤" blue, Assistant: "⚡" green, System: "ℹ" orange
- Debug: "🔧" gray, JsonRequest/JsonResponse: "📤📥" orange
- egui::Color32 used for cross-platform rendering

---

**Last Updated**: 2025-01-28
**Status**: Accurately reflects message.rs implementation
