# Callback Handlers - Tool Execution to UI Event Conversion

## Component Overview

Converts stood agent callbacks (tool execution events, model requests) into
AgentResponse messages for UI display. Creates tree-structured message
representation and captures JSON debugging data.

**Pattern**: Observer pattern with message transformation
**External**: stood callback trait system, mpsc channels
**Purpose**: Decouple agent execution from UI rendering

---

## Major Types

### AgentToolCallbackHandler
- Handles tool start/complete/failed events
- Creates parent-child message tree structure
- Maps tool names to user-friendly action labels
- Logs to per-agent logger

### JsonCaptureHandler
- Captures model request/response JSON
- Pretty-prints for debugging
- Stores raw JSON when available
- Sends to UI via AgentResponse::JsonDebug

---

## Implementation Patterns

### Pattern: Tool Event to Message Tree

**Algorithm**: Parent message on start, child message on complete
**External**: HashMap for active tool tracking, mpsc::Sender

Pseudocode:
  1. ToolEvent::Started received:
     - Create unique tool_node_id (tool_name + timestamp)
     - Map tool name to friendly action (e.g., "List" for aws_list_resources)
     - Create parent Message with role=System, content=friendly_name
     - Create nested Message with role=JsonRequest, content=pretty_json(input)
     - Store tool_node_id in active_tool_nodes HashMap
     - Send AgentResponse::ToolCallStart to UI
  2. ToolEvent::Completed received:
     - Find parent_node_id from active_tool_nodes by tool name
     - Create child Message with role=Assistant, content=completion_status
     - Create nested Message with role=JsonResponse, content=pretty_json(output)
     - Send AgentResponse::ToolCallComplete to UI
     - Remove from active_tool_nodes HashMap
  3. UI builds tree: parent.nested_messages.push(child)

### Pattern: User-Friendly Action Mapping

**Algorithm**: Static mapping from tool names to display labels
**External**: match expression on tool name

Pseudocode:
  1. Define mapping:
     - aws_list_resources → "List"
     - aws_describe_resource → "Describe"
     - aws_find_account → "Find Account"
     - create_task → "Task"
     - search_logs → "Search Logs"
     - Default → "Tool"
  2. For create_task: extract task_description, show as main content
  3. For other tools: show friendly action name
  4. Enables concise UI display without exposing internals

### Pattern: JSON Debug Capture

**Algorithm**: Intercept ModelStart/ModelComplete callbacks
**External**: CallbackEvent enum, serde_json

Pseudocode:
  1. CallbackEvent::ModelStart:
     - Extract provider, model_id, messages, tools_available
     - Serialize to JSON with serde_json::to_string_pretty
     - Create JsonDebugData with type=Request, timestamp=now
     - Send AgentResponse::JsonDebug to UI
  2. CallbackEvent::ModelComplete:
     - Extract response, stop_reason, duration, tokens
     - Serialize to JSON (include token usage)
     - Create JsonDebugData with type=Response
     - Send to UI
  3. UI displays in separate JSON debug panel

---

## External Dependencies

- **stood**: CallbackHandler trait, CallbackEvent, ToolEvent
- **serde_json**: JSON pretty-printing
- **mpsc**: Channel for UI communication
- **AgentLogger**: Per-agent execution logging

---

## Key Algorithms

### Tool Call Matching
- Use tool name + timestamp as unique key
- Handle multiple simultaneous tool calls
- Most recent tool with name X matched on completion
- Edge case: parallel same-tool calls use timestamp ordering

### Message ID Generation
- Format: "tool_{name}_{timestamp_millis}"
- Ensures uniqueness for tree linking
- Chronological ordering from timestamps

---

**Last Updated**: 2025-10-28
