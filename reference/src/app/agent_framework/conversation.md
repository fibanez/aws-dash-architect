# Conversation Module - Agent Message Types

## Component Overview

Defines message types for agent conversations. These types represent the
communication protocol between agents and the UI layer.

**Pattern**: Typed message protocol
**External**: serde for serialization
**Purpose**: Structured agent-UI communication

---

## Key Types

### ConversationMessage
Represents a single message in the conversation:
- `role`: ConversationRole (User, Assistant, System)
- `content`: String message content
- `timestamp`: When message was created
- `metadata`: Optional additional data

### ConversationRole
Enum for message sender:
- `User` - Human input
- `Assistant` - Agent response
- `System` - System notifications

### ConversationResponse
Response from agent execution:
- `message`: The response message
- `token_usage`: Optional token consumption data
- `tool_calls`: List of tools invoked

---

## Usage Pattern

```rust
// Create user message
let user_msg = ConversationMessage {
    role: ConversationRole::User,
    content: "List EC2 instances".to_string(),
    timestamp: Utc::now(),
    metadata: None,
};

// Agent responds
let response = ConversationResponse {
    message: ConversationMessage {
        role: ConversationRole::Assistant,
        content: "Found 5 EC2 instances...".to_string(),
        timestamp: Utc::now(),
        metadata: None,
    },
    token_usage: Some(usage),
    tool_calls: vec![],
};
```

---

**Last Updated**: 2025-12-22
**Status**: Accurately reflects conversation.rs
