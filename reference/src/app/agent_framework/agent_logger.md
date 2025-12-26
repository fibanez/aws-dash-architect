# Agent Logger - Per-Agent Execution Logs

## Component Overview

Per-agent logging system. Each AgentInstance has dedicated log file at
~/.local/share/awsdash/logs/agents/agent-{uuid}.log tracking conversation,
model interactions, tool executions, and lifecycle events. Worker agents can
share parent's log file for unified conversation tracking.

**Pattern**: File-based structured logging per agent
**Algorithm**: Buffered file writes with session headers
**External**: std::fs::File, directories crate for platform paths

---

## Major Methods

- `new(agent_id, name, agent_type)` - Create logger with session header
- `log_path()` - Get log file path for UI display
- `update_agent_name()` - Log agent rename event
- `log_agent_created(agent_type, metadata)` - Log creation with type info
- `log_agent_terminated()` - Log final status
- `log_user_message(agent_type, message)` - Log user input with type prefix
- `log_assistant_response(agent_type, response)` - Log agent response
- `log_system_message(agent_type, message)` - Log system events
- `log_model_changed(agent_type, old, new)` - Log model change
- `log_model_request()` - Log LLM request (prompt, input, model_id)
- `log_model_response()` - Log LLM response (output, tokens, duration)
- `log_tool_execution_start()` - Log tool invocation
- `log_tool_execution_success()` - Log tool completion
- `log_tool_execution_failed()` - Log tool error
- `log_error(agent_type, error)` - Log error events with type prefix

---

## Implementation Patterns

### Pattern: Dedicated Log File Per Agent

**Algorithm**: Platform-specific log directory with agent UUID
**External**: directories::ProjectDirs for platform paths

Pseudocode:
  1. get_log_path(agent_id):
     - ProjectDirs::from("com", "", "awsdash")
     - data_dir/logs/agents/agent-{uuid}.log
     - Create parent directories if missing
  2. new(agent_id, name):
     - Open file in append mode
     - Write session header with "=" separators
     - Log agent ID, name, timestamp
     - Flush to disk
  3. All log methods lock file_writer Mutex, write, flush

### Pattern: Session-Based Logging

**Algorithm**: Session headers separate agent restarts
**External**: chrono::Utc for timestamps

Pseudocode:
  1. Session header on new():
     "\n==============================\n"
     "AGENT SESSION STARTED: {timestamp}\n"
     "Agent ID: {id}\n"
     "Agent Name: {name}\n"
     "Agent Type: {TaskManager|TaskWorker}\n"
     "==============================\n"
  2. All log entries timestamped
  3. Session allows distinguishing multiple runs in single file

### Pattern: Worker Log Sharing

**Algorithm**: Workers use parent's logger for unified tracking
**External**: Arc<AgentLogger> shared reference

Pseudocode:
  1. TaskManager creates own AgentLogger via new()
  2. When spawning TaskWorker:
     - new_with_parent_logger(metadata, type, parent.logger())
     - Worker uses parent's Arc<AgentLogger>
  3. Worker logs appear in parent's log file:
     "\n====== Worker Agent: {name} ({id}) ======"
  4. Complete conversation flow in single file
  5. AgentType prefix in log entries distinguishes messages

### Pattern: Structured Model Logging

**Algorithm**: Request/response pairs with token tracking
**External**: TokenUsage struct

Pseudocode:
  1. log_model_request(system_prompt, user_input, model_id):
     - Log "📤 MODEL REQUEST" header
     - Log model ID, prompt length, input length
     - Optionally log full prompt (truncated for large prompts)
  2. log_model_response(output, status, duration_ms, tokens):
     - Log "📥 MODEL RESPONSE" header
     - Log status, output length, duration
     - Log token usage: input_tokens, output_tokens, total_tokens
     - Calculate cost estimates if rates available

### Pattern: Tool Execution Tracking

**Algorithm**: Start/success/failed triples with timing
**External**: Duration for elapsed time

Pseudocode:
  1. log_tool_execution_start(tool_name, input):
     - Log "🔧 TOOL START: {tool_name}" with timestamp
     - Log input parameters as JSON
  2. log_tool_execution_success(tool_name, output, duration):
     - Log "✅ TOOL SUCCESS: {tool_name}" with duration
     - Log output (truncated if large)
  3. log_tool_execution_failed(tool_name, error, duration):
     - Log "❌ TOOL FAILED: {tool_name}" with duration
     - Log error message

---

## External Dependencies

- **std::fs**: File, OpenOptions for log file management
- **std::sync**: Arc<Mutex<File>> for thread-safe writes
- **std::io::Write**: flush() after writes
- **directories**: Platform-specific data directory paths
- **chrono**: UTC timestamps
- **tracing**: Fallback logging on file errors

---

## Key Algorithms

### Log File Path Resolution
Platform-specific paths via directories crate:
- Linux: ~/.local/share/awsdash/logs/agents/
- macOS: ~/Library/Application Support/awsdash/logs/agents/
- Windows: %APPDATA%\awsdash\logs\agents\

### Buffered Writes with Flush
Lock Mutex → write → flush → unlock
Ensures logs persisted immediately for debugging

---

**Last Updated**: 2025-11-25
**Status**: Updated for multi-agent system (AgentType prefix, worker log sharing)
