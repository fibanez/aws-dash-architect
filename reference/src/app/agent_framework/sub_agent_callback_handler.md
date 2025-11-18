# Sub-Agent Callback Handler (DEPRECATED)

## Component Overview

Deprecated stub callback handler kept for backward compatibility.
Agents now handle their own event loops without streaming callbacks.
All methods are no-ops that log and return Ok(()).

**Pattern**: Stub/no-op implementation of CallbackHandler trait
**Algorithm**: None - all callbacks ignored
**External**: stood::agent::callbacks::CallbackHandler

---

## Major Methods

- `new()` - Create standalone handler (no sender)
- `with_sender()` - Create handler with sender (sender ignored)
- `handle_event()` - No-op, logs and returns Ok(())

---

## Implementation Notes

### Deprecation Reason
Original design: streaming callbacks for sub-agent events
Current design: agents handle execution internally via AgentInstance
Kept for backward compatibility with existing tool integrations

### Current Behavior
All CallbackEvent variants ignored:
- handle_event() logs event type via debug!()
- Returns Ok(()) immediately
- No state changes, no message sending
- Agent execution handled by AgentInstance::send_message()

---

## External Dependencies

- **stood::agent::callbacks**: CallbackHandler trait, CallbackEvent
- **async_trait**: #[async_trait] macro
- **tracing**: debug! logging

---

**Last Updated**: 2025-01-28
**Status**: Accurately reflects sub_agent_callback_handler.rs (deprecated stub)
