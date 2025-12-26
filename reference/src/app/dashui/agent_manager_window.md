# Agent Manager Window - Reference Documentation

## Component Overview

Unified two-pane UI for managing multiple AI agents. Left pane shows agent
list with creation controls. Right pane shows selected agent's chat view
with markdown-rendered responses.

---

## Major Components

| Component | Description |
|-----------|-------------|
| `AgentManagerWindow` | Main window struct with agents, selection, and state |
| `TaskContext` | Context for displaying task progress (worker cycling) |
| `WorkerTabMetadata` | Auto-close timer tracking for completed workers |

---

## Layout Structure

```
+------------------------+----------------------------------+
|  Left Pane (130px)     |  Right Pane (remainder)          |
|------------------------|----------------------------------|
|  Heading: "Agents"     |  Task Indicator (if worker)      |
|  Log Level: [Dropdown] |  Tab Bar (Manager | Worker N)    |
|  Model: [Dropdown]     |  Conversation (ScrollArea)       |
|  [+ New Agent]         |  Status Line                     |
|  ---separator---       |  Input Area (3 rows)             |
|  Agent List (scroll)   |  [Send] Button                   |
|    - Agent 1           |  Action Buttons: Log|Clear|Term  |
|    - Agent 2           |                                  |
|    - ...               |                                  |
+------------------------+----------------------------------+
```

---

## Implementation Patterns

**Two-Pane Layout**:
- Pattern: egui_extras StripBuilder
- Left: fixed 130px, Right: remainder
- Both cells fill vertical space

**Agent Selection**:
- Pattern: Option<AgentId> for selection state
- Tab selection: separate from agent selection for workers
- Maintains scroll position per agent

**Markdown Rendering**:
- Pattern: Shared CommonMarkCache across all agents
- Passed to render_agent_chat() for message rendering
- Avoids re-parsing markdown on each frame

**Worker Auto-Close**:
- Pattern: Timer-based cleanup
- 30-second countdown after worker completion
- Viewing resets timer (preserves user interest)

---

## External Dependencies

| Dependency | Purpose |
|------------|---------|
| egui | UI framework |
| egui_extras | StripBuilder for layout |
| egui_commonmark | Markdown rendering cache |
| chrono | Timestamps for agents |

---

## Pseudocode

### ui_content()

```
1. Create StripBuilder with two columns:
   - Left: Size::exact(130.0)
   - Right: Size::remainder()

2. Left pane cell:
   a. Heading "Agents"
   b. Log level dropdown (StoodLogLevel)
   c. Model selector dropdown (AgentModel::all_models)
   d. "+ New Agent" button
   e. Separator
   f. ScrollArea with agent list
      - For each agent: selectable_label
      - Track clicked agent for selection

3. Right pane cell:
   a. If agent selected: render_agent_chat_view(agent_id)
   b. Else: blank space (no message)
```

### create_new_agent()

```
1. Generate name: "Agent N" where N = count + 1
2. Create AgentMetadata with:
   - name, description, model (from selected_model)
   - created_at, updated_at = now

3. Create AgentInstance(metadata, AgentType::TaskManager)
4. Set stood_log_level on agent
5. Initialize with AWS credentials
6. Insert into agents HashMap
7. Select the new agent
```

### render_agent_chat_view()

```
Input: ui, agent_id

1. Render task indicator (if task-worker)
2. Build tab list: [manager_id] + worker_ids
3. Show tab bar if workers exist OR manager has messages

4. Determine display_agent_id:
   - With workers: use selected_tab_agent_id
   - Without workers: use agent_id

5. Get agent for display_agent_id
6. Call render_agent_chat(ui, agent, input, markdown_cache)

7. Handle button actions:
   - should_send: agent.send_message() + clear input
   - log_clicked: open AgentLogWindow
   - clear_clicked: agent.clear_conversation()
   - terminate_clicked: remove from agents map
```

### poll_agent_responses_global()

```
Called every frame from DashApp::update()

1. Increment frame counter for logging
2. For each agent in agents:
   a. Call agent.poll_response()
   b. If response received:
      - Log timing info
      - Check if completed worker
      - Collect completed workers

3. For each completed worker:
   a. Calculate execution time
   b. Send WorkerCompletion to channel
   c. Mark worker tab as completed (start 30s timer)

4. Auto-close expired worker tabs:
   - Check should_auto_close() for each
   - Remove agent and tab metadata
   - Clear selection if removed agent was selected
```

---

## Event Processing

### UI Events (process_ui_events)

| Event | Action |
|-------|--------|
| SwitchToAgent(id) | select_agent(id) |
| SwitchToParent(id) | select_agent(id) |
| AgentCompleted(id) | handle_agent_completion(id) |

### Agent Creation Requests (process_agent_creation_requests)

```
1. Collect pending requests from channel
2. For each request:
   a. Verify parent exists
   b. Create TaskWorker with parent's model
   c. Initialize with AWS credentials
   d. Send initial task message
   e. Send success/error response via channel
```

---

## Design Decisions

**Why Global Polling**:
- Ensures agent responses are received even when window closed
- Called from DashApp::update() before rendering
- Prevents missed messages during UI navigation

**Why CommonMarkCache Shared**:
- Single cache for all agents reduces memory
- Markdown rarely changes once rendered
- Passed by reference, not cloned

**Why 30-Second Auto-Close**:
- Gives users time to review completed workers
- Viewing resets timer (user interest preservation)
- Automatic cleanup prevents tab clutter

**Why Tab System for Workers**:
- Shows all related conversations in context
- Manager tab always first for consistency
- Easy switching without leaving agent

---

## Related Files

- `src/app/agent_framework/agent_ui.rs` - render_agent_chat() function
- `src/app/agent_framework/agent_instance.rs` - AgentInstance struct
- `src/app/agent_framework/model_selection.rs` - AgentModel enum
- `src/app/dashui/agent_log_window.rs` - Log viewer window

---

**Last Updated**: 2025-12-23
