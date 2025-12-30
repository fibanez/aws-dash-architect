# Agent Feedback Systems - Integration Testing Guide

This document provides step-by-step manual testing procedures for the Agent Feedback Systems implemented in Phases A, B, and C.

## Prerequisites

1. **AWS Bedrock Access**: Ensure your AWS account has Anthropic Claude model access configured
   - Go to AWS Bedrock Console > Model access
   - Request access to Anthropic Claude models
   - Wait for approval (can take 15+ minutes)

2. **Valid AWS Credentials**: Ensure SSO login is active
   ```bash
   aws sso login --profile <your-profile>
   ```

3. **Build the Application**:
   ```bash
   cargo build
   cargo run
   ```

---

## Part 1: Status Display Animations

### Test 1.1: Orbital Dots (Thinking Phase)

**Objective**: Verify orbital dots animation appears when agent starts processing.

**Steps**:
1. Open the application
2. Create a new agent (Task Manager)
3. Type a simple message: "Hello, what can you help me with?"
4. Press Enter or click Send

**Expected Result**:
- [ ] Orbital dots animation appears immediately
- [ ] Animation shows 3 dots rotating in a circle
- [ ] Whimsical message appears (e.g., "Pondering possibilities...", "Cogitating carefully...")
- [ ] "Processing..." appears as detail text

**Pass/Fail**: ______

---

### Test 1.2: Wave Bars (Tool Execution Phase)

**Objective**: Verify wave bars animation appears during tool execution.

**Note**: This requires stood library to send StatusUpdate messages. Currently, stood does NOT send these, so wave bars may not appear. This test documents expected behavior for future implementation.

**Steps**:
1. Open the application
2. Create a new agent (Task Manager)
3. Send a message that triggers tool use: "List all EC2 instances in us-east-1"
4. Observe the status display during processing

**Expected Result** (when stood sends StatusUpdate):
- [ ] Orbital dots initially (thinking)
- [ ] Transitions to wave bars during tool execution
- [ ] Wave bars show 5 animated vertical bars
- [ ] Message changes to tool-related (e.g., "Running tool...")

**Current Behavior** (stood doesn't send StatusUpdate):
- [ ] Only orbital dots shown throughout processing

**Pass/Fail**: ______ (Note: Expected to show only orbital dots currently)

---

### Test 1.3: Idle State

**Objective**: Verify animation stops when processing completes.

**Steps**:
1. Complete any agent request
2. Wait for response to appear

**Expected Result**:
- [ ] Animation stops completely
- [ ] No status message shown
- [ ] Response is displayed in conversation

**Pass/Fail**: ______

---

### Test 1.4: Whimsical Messages Rotation

**Objective**: Verify messages rotate during long processing.

**Steps**:
1. Send a complex request that takes several seconds
2. Watch the status message area

**Expected Result**:
- [ ] Message changes every few seconds
- [ ] Messages match the current phase
- [ ] Messages are varied (not always the same)

**Pass/Fail**: ______

---

## Part 2: Markdown Table ID Bug Fix

### Test 2.1: Multiple Responses with Tables

**Objective**: Verify tables in different responses don't cause ID conflicts.

**Steps**:
1. Open the application
2. Create a new agent
3. Send: "Show me a simple table with 3 columns: Name, Value, Status"
4. Wait for response with table
5. Send: "Show me another table with columns: ID, Description, Count"
6. Wait for second response with table

**Expected Result**:
- [ ] First table renders correctly
- [ ] Second table renders correctly
- [ ] NO red warning/error box appears in the UI
- [ ] Both tables maintain proper formatting

**Previous Bug Behavior** (before fix):
- Red warning about duplicate widget IDs
- Tables might not render correctly

**Pass/Fail**: ______

---

### Test 2.2: Same Response with Multiple Tables

**Objective**: Verify multiple tables in single response work correctly.

**Steps**:
1. Send: "Create two tables: one for fruits (name, color) and one for vegetables (name, type)"

**Expected Result**:
- [ ] Both tables render in the same response
- [ ] No ID conflicts
- [ ] Proper spacing between tables

**Pass/Fail**: ______

---

## Part 3: Message Injection System

### Test 3.1: Injection API Availability

**Objective**: Verify injection methods are accessible (code-level test).

**Note**: Message injection is an API feature. Testing requires either:
- Code modification to queue injections
- Future UI integration

**Verification** (for developers):
```rust
// These methods should exist on AgentInstance:
agent.queue_injection(InjectionType::SystemContext("test".into()), InjectionTrigger::Immediate);
agent.queue_immediate_injection(InjectionType::Correction("test".into()));
agent.inject_message("Test message".into());
agent.has_pending_injections();
```

**Expected Result**:
- [ ] All injection methods compile without errors
- [ ] No runtime errors when calling methods

**Pass/Fail**: ______

---

### Test 3.2: AfterResponse Injection Trigger

**Objective**: Verify injections can be triggered after responses.

**Setup** (requires code modification):
```rust
// In your test code, before sending a message:
agent.queue_injection(
    InjectionType::ToolFollowUp {
        tool_name: "test".into(),
        context: "Analyze the results".into(),
    },
    InjectionTrigger::AfterResponse,
);
```

**Steps**:
1. Queue an AfterResponse injection
2. Send a regular message
3. Wait for response
4. Observe if follow-up is automatically sent

**Expected Result**:
- [ ] Original response appears
- [ ] Follow-up message is automatically sent
- [ ] Agent processes the follow-up

**Pass/Fail**: ______ (Requires code integration)

---

## Part 4: Middleware System

### Test 4.1: Middleware Infrastructure

**Objective**: Verify middleware components are available.

**Note**: Middleware is infrastructure-only. Full integration requires wiring into agent execution path.

**Verification** (for developers):
```rust
use agent_framework::middleware::{LayerStack, TokenTrackingLayer, AutoAnalysisLayer};

let mut stack = LayerStack::new();
stack.add(TokenTrackingLayer::with_defaults());
stack.add(AutoAnalysisLayer::with_defaults());
```

**Expected Result**:
- [ ] All middleware types compile
- [ ] Layers can be added to stack
- [ ] No runtime errors

**Pass/Fail**: ______

---

## Part 5: Error Scenarios

### Test 5.1: API Error Handling

**Objective**: Verify graceful handling of Bedrock API errors.

**Steps** (if Bedrock not configured):
1. Attempt to use agent without proper Bedrock access
2. Send any message

**Expected Result**:
- [ ] Clear error message displayed
- [ ] Agent status changes to Failed
- [ ] Application doesn't crash
- [ ] User can see error details

**Pass/Fail**: ______

---

### Test 5.2: Long Response Handling

**Objective**: Verify UI handles long responses correctly.

**Steps**:
1. Send: "Write a detailed 500-word explanation of how AWS Lambda works"
2. Wait for response

**Expected Result**:
- [ ] Response appears without truncation
- [ ] Scroll works correctly
- [ ] Markdown formatting preserved
- [ ] No UI freezing

**Pass/Fail**: ______

---

## Test Summary

| Test ID | Test Name | Status | Notes |
|---------|-----------|--------|-------|
| 1.1 | Orbital Dots | | |
| 1.2 | Wave Bars | | Expected: orbital only (stood limitation) |
| 1.3 | Idle State | | |
| 1.4 | Message Rotation | | |
| 2.1 | Multiple Tables | | |
| 2.2 | Same Response Tables | | |
| 3.1 | Injection API | | Code-level test |
| 3.2 | AfterResponse Injection | | Requires integration |
| 4.1 | Middleware Infrastructure | | Code-level test |
| 5.1 | API Error Handling | | |
| 5.2 | Long Response | | |

---

## Known Limitations

1. **Wave Bars Animation**: Currently won't appear because stood library doesn't emit StatusUpdate messages during tool execution. Orbital dots shown throughout processing.

2. **Message Injection**: API is available but not integrated into UI. Requires code-level usage.

3. **Middleware**: Infrastructure complete but not wired into agent execution flow.

4. **Analyzing Phase**: The "slower orbital" animation for analyzing results is implemented but won't trigger without stood changes.

---

## Troubleshooting

### Animation Not Appearing
- Check if agent is actually processing (`is_processing()` returns true)
- Verify no errors in application log: `~/.local/share/awsdash/logs/awsdash.log`

### Red Warning on Tables
- If still appearing after fix, check message timestamps are unique
- Verify `ui.push_id()` is wrapping the markdown viewer

### Bedrock Errors
- Check AWS credentials: `aws sts get-caller-identity`
- Verify model access in Bedrock console
- Check agent log: `~/.local/share/awsdash/logs/agents/*.log`

---

## Version Information

- **Branch**: FEEDBACK-from-AGENTS
- **Features Tested**:
  - Phase A: Status Display Engine
  - Phase B: Message Injection Engine
  - Phase C: Conversation Middleware
- **Date**: December 2025
