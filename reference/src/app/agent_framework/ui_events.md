# UI Events - Global Event Channel for Agent Tools

## Component Overview

Provides a global event channel for agent tools to trigger UI state changes
without requiring direct access to AgentManagerWindow. Uses mpsc channel pattern
with OnceLock for lazy initialization.

**Pattern**: Publish-subscribe with global channel
**Algorithm**: mpsc channel, OnceLock lazy initialization
**External**: std::sync::mpsc, std::sync::OnceLock, std::sync::Mutex

---

## Major Types

### AgentUIEvent Enum
- `SwitchToAgent(AgentId)` - Switch UI to display specific agent
- `SwitchToParent(AgentId)` - Return to parent agent after worker completes
- `AgentCompleted(AgentId)` - Notify UI that agent finished work

---

## Major Functions

- `init_ui_event_channel()` - Initialize global channel (called on first access)
- `get_ui_event_sender()` - Get Sender clone for tools to send events
- `get_ui_event_receiver()` - Get Arc<Mutex<Receiver>> for UI polling
- `send_ui_event()` - Convenience helper to send event directly

---

## Implementation Patterns

### Pattern: Global Event Channel

**Algorithm**: OnceLock with Sender/Arc<Mutex<Receiver>> pair
**External**: std::sync::mpsc, OnceLock

Pseudocode:
  1. UI_EVENT_CHANNEL: OnceLock<(Sender, Arc<Mutex<Receiver>>)>
  2. get_ui_event_sender():
     - get_or_init() creates channel pair if first call
     - Returns cloned Sender (Sender is Clone)
  3. get_ui_event_receiver():
     - Returns Arc clone of Mutex<Receiver>
     - UI must lock() to poll for events
  4. Events processed in AgentManagerWindow::process_ui_events()

### Pattern: Decoupled Tool-to-UI Communication

**Algorithm**: Fire-and-forget events from tool threads
**External**: mpsc::Sender::send()

Pseudocode:
  1. Tool (e.g., start_task) executes on background thread
  2. Tool cannot access AgentManagerWindow directly
  3. Tool sends event: send_ui_event(AgentUIEvent::switch_to_agent(id))
  4. UI polls receiver each frame via try_recv()
  5. UI processes events and updates state

### Pattern: Event Ordering Guarantee

**Algorithm**: FIFO queue (mpsc channel semantics)
**External**: std::sync::mpsc guarantees

Pseudocode:
  1. Multiple events sent in order
  2. Receiver gets events in same order
  3. Critical for: SwitchToAgent -> AgentCompleted sequences

---

## External Dependencies

- **std::sync::mpsc** - Multi-producer single-consumer channel
- **std::sync::OnceLock** - Lazy initialization
- **std::sync::Arc/Mutex** - Shared receiver (Receiver is not Clone)
- **AgentId** - Agent identifier for event targeting

---

## Key Algorithms

### Channel Initialization
Lazy via OnceLock::get_or_init(), thread-safe singleton pattern

### Event Types Use Cases
- SwitchToAgent: Worker spawned, show in UI
- SwitchToParent: Worker done, return to manager view
- AgentCompleted: Update status indicators, cleanup

---

**Last Updated**: 2025-11-25
**Status**: New file for multi-agent task orchestration system
