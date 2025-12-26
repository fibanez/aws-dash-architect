# Agent Creation - Channel-Based Agent Spawning System

## Component Overview

Provides a channel-based system for tools (like start-task) to request agent
creation from AgentManagerWindow without requiring direct window access. Uses
global channels with request/response pattern for decoupled communication.

**Pattern**: Request/Response channel pattern with unique request IDs
**Algorithm**: Global OnceLock channels, mpsc for sync communication
**External**: std::sync::mpsc, std::sync::OnceLock, std::sync::Mutex

---

## Major Types

- `AgentCreationRequest` - Request to create new agent with task description
- `AgentCreationResponse` - Response with agent_id, success flag, error
- `AGENT_CREATION_CHANNEL` - Global sender/receiver for requests
- `AGENT_CREATION_RESPONSES` - HashMap of request_id to response senders
- `REQUEST_ID_COUNTER` - Atomic counter for unique request IDs

---

## Major Functions

- `init_agent_creation_channel()` - Initialize global channel (lazy or explicit)
- `get_agent_creation_sender()` - Get clone of request sender for tools
- `get_agent_creation_receiver()` - Get Arc<Mutex<Receiver>> for UI
- `take_response_channel()` - Remove and return response sender for request_id
- `request_agent_creation()` - Convenience: send request, block for response

---

## Implementation Patterns

### Pattern: Request/Response with Unique IDs

**Algorithm**: Each request gets unique ID, registers one-shot response channel
**External**: mpsc::channel, HashMap, Mutex

Pseudocode:
  1. AgentCreationRequest::new(task, format, parent_id):
     - Generate unique request_id via next_request_id()
     - Create one-shot (sender, receiver) channel
     - Register sender in AGENT_CREATION_RESPONSES HashMap
     - Return (request, response_receiver)
  2. Tool sends request via get_agent_creation_sender().send(request)
  3. AgentManagerWindow receives via get_agent_creation_receiver()
  4. UI creates agent, gets request_id from request
  5. UI calls take_response_channel(request_id) to get response sender
  6. UI sends AgentCreationResponse via sender
  7. Tool receives response via response_receiver

### Pattern: Lazy Global Initialization

**Algorithm**: OnceLock with get_or_init() pattern
**External**: std::sync::OnceLock

Pseudocode:
  1. AGENT_CREATION_CHANNEL uses OnceLock
  2. get_agent_creation_sender() calls get_or_init()
  3. If not initialized: create channel pair
  4. Return cloned sender (Sender is Clone)
  5. Receiver wrapped in Arc<Mutex<>> (not Clone)

### Pattern: Blocking Request Helper

**Algorithm**: Send request, wait with timeout
**External**: recv_timeout()

Pseudocode:
  1. request_agent_creation(task, format, parent_id):
     - Create request via AgentCreationRequest::new()
     - Send via get_agent_creation_sender()
     - recv_timeout(5 seconds) on response channel
     - Return Ok(agent_id) or Err(error)

---

## External Dependencies

- **std::sync::mpsc** - Channel for request/response communication
- **std::sync::OnceLock** - Lazy global initialization
- **std::sync::Arc/Mutex** - Shared receiver access
- **std::collections::HashMap** - Request ID to response sender mapping
- **AgentId** - Unique agent identifier

---

## Key Algorithms

### Request ID Generation
Atomic counter using Arc<Mutex<u64>>, increments on each call

### Channel Architecture
- Request channel: global, multi-producer (tools), single-consumer (UI)
- Response channels: per-request, single-producer (UI), single-consumer (tool)
- Decouples tool execution from UI thread

---

**Last Updated**: 2025-11-25
**Status**: New file for multi-agent task orchestration system
