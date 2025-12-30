# Agent Injection Patterns - Research & Use Cases

This document captures industry research on programmatic message injection, context management, and memory patterns for AI agents. Use this as a reference when implementing new injection use cases.

## Table of Contents

1. [Industry Sources](#industry-sources)
2. [Anthropic Patterns](#anthropic-patterns)
3. [Andrew Ng's Agentic Patterns](#andrew-ngs-agentic-patterns)
4. [Context Engineering Strategies](#context-engineering-strategies)
5. [Memory Management Patterns](#memory-management-patterns)
6. [Claude Code Design Patterns](#claude-code-design-patterns)
7. [Use Case Mapping to Our API](#use-case-mapping-to-our-api)
8. [Future Implementation Ideas](#future-implementation-ideas)

---

## Industry Sources

### Anthropic

| Resource | URL | Key Topics |
|----------|-----|------------|
| Building Effective Agents | [anthropic.com/engineering/building-effective-agents](https://www.anthropic.com/engineering/building-effective-agents) | 5 workflow patterns, agent design |
| Context Engineering Guide | [the-decoder.com](https://the-decoder.com/anthropic-claims-context-engineering-beats-prompt-engineering-when-managing-ai-agents/) | Compacting, structured notes, sub-agents |
| Prompt Injection Defenses | [anthropic.com/research/prompt-injection-defenses](https://www.anthropic.com/research/prompt-injection-defenses) | Security patterns |

### Andrew Ng / DeepLearning.AI

| Resource | URL | Key Topics |
|----------|-----|------------|
| Agentic Reasoning 2024 | [octetdata.com](https://octetdata.com/blog/notes-andrew-ng-agentic-reasoning-2024/) | 4 design patterns |
| Agentic AI Course | [deeplearning.ai/courses/agentic-ai](https://www.deeplearning.ai/courses/agentic-ai/) | Production patterns |
| AI Roundup 2024 | [analyticsvidhya.com](https://www.analyticsvidhya.com/blog/2025/01/ai-roundup-by-andrew-ng/) | Trend analysis |

### Memory & Context Research

| Resource | URL | Key Topics |
|----------|-----|------------|
| LangChain Context Engineering | [blog.langchain.com/context-engineering-for-agents](https://blog.langchain.com/context-engineering-for-agents/) | 4 context strategies |
| LangGraph Memory | [blog.langchain.com/memory-for-agents](https://blog.langchain.com/memory-for-agents/) | Short/long-term memory |
| Memory Survey (arXiv) | [arxiv.org/abs/2404.13501](https://arxiv.org/abs/2404.13501) | Academic survey |
| Mem0 Agent Memory | [mem0.ai/blog/memory-in-agents-what-why-and-how](https://mem0.ai/blog/memory-in-agents-what-why-and-how) | Memory implementation |
| MemGPT Paper | [emergentmind.com](https://www.emergentmind.com/topics/persistent-memory-for-llm-agents) | Two-tier memory |

### Claude Code Analysis

| Resource | URL | Key Topics |
|----------|-----|------------|
| Agent Design Lessons | [jannesklaas.github.io](https://jannesklaas.github.io/ai/2025/07/20/claude-code-agent-design.html) | TODO, sub-agents, reminders |
| Running for Hours | [motlin.com](https://motlin.com/blog/claude-code-running-for-hours) | Long-running patterns |
| Workflow Tips | [thegroundtruth.substack.com](https://thegroundtruth.substack.com/p/my-claude-code-workflow-and-personal-tips) | ROADMAP.md patterns |

---

## Anthropic Patterns

### Five Workflow Patterns

From [Building Effective Agents](https://www.anthropic.com/engineering/building-effective-agents):

#### 1. Prompt Chaining

> "Each LLM call processes the output of the previous one"

**How it works**: Decompose tasks into sequential steps with programmatic "gates" to verify intermediate steps.

**Use cases**:
- Marketing copy generation followed by translation
- Document outlining with validation before full writing
- Multi-step data processing pipelines

**Our API mapping**:
```rust
InjectionTrigger::AfterResponse
```

#### 2. Routing

> "Classifies an input and directs it to a specialized followup task"

**How it works**: LLM router classifies input and routes to specialized handlers.

**Use cases**:
- Customer service (general/refund/technical)
- Routing easy questions to Haiku, complex to Sonnet
- Multi-domain query handling

**Our API mapping**: Not directly applicable (orchestration-level concern)

#### 3. Parallelization

> "Multiple LLM calls run simultaneously with programmatic aggregation"

**Variants**:
- **Sectioning**: Independent subtasks run in parallel
- **Voting**: Same task multiple times for consensus

**Use cases**:
- Guardrails (separate safety screening instances)
- Code vulnerability reviews from multiple angles
- Content moderation with threshold voting

**Our API mapping**: Not applicable (injection is sequential)

#### 4. Orchestrator-Workers

> "Central LLM dynamically breaks tasks, delegates to workers, synthesizes results"

**How it works**: Unlike parallelization, subtasks aren't pre-defined but determined dynamically.

**Use cases**:
- Complex code changes across multiple files
- Multi-source information gathering
- Research tasks with unknown scope

**Our API mapping**:
```rust
InjectionType::WorkerResult { worker_id, result }
```

#### 5. Evaluator-Optimizer

> "One LLM generates responses; another provides feedback in loops"

**How it works**: Iterative refinement until quality threshold met.

**Use cases**:
- Literary translation refinement
- Complex search with multiple analysis rounds
- Code review and improvement cycles

**Our API mapping**:
```rust
InjectionType::Correction(feedback)
InjectionTrigger::AfterResponse
```

### Context Engineering Tactics

From Anthropic's context engineering research:

| Tactic | Description | Our API |
|--------|-------------|---------|
| **Compacting** | Summarize conversations near context limit | `MemorySummary` + `OnTokenThreshold` |
| **Structured Notes** | Persist info outside context window | `SystemContext` |
| **Sub-agent Architectures** | Delegate to focused agents, get summaries | `WorkerResult` |

---

## Andrew Ng's Agentic Patterns

From [Agentic Reasoning 2024](https://octetdata.com/blog/notes-andrew-ng-agentic-reasoning-2024/):

### 1. Reflection

> "LLM reviews and improves its own output"

**Implementation**: Prompt model to "check for correctness, soundness, efficiency" then refine through multiple passes.

**Our API mapping**:
```rust
agent.queue_injection(
    InjectionType::Correction(
        "Review your response for correctness and efficiency. \
         If you find issues, provide a corrected version.".into()
    ),
    InjectionTrigger::AfterResponse,
);
```

### 2. Tool Use

> "Expand capabilities through external resources"

**Examples**: Web searches, code execution, API interactions.

**Our API mapping**:
```rust
InjectionTrigger::AfterToolComplete { tool_name, on_success_only }
```

### 3. Planning

> "Break complex tasks into subtasks with sequential execution"

**Note**: Ng says this is "incredibly powerful, but works less consistently."

**Our API mapping**:
```rust
// Queue sequence with priorities
agent.queue(PendingInjection::new(step1, trigger).with_priority(3));
agent.queue(PendingInjection::new(step2, trigger).with_priority(2));
agent.queue(PendingInjection::new(step3, trigger).with_priority(1));
```

### 4. Multi-Agent Collaboration

> "Multiple specialized agents with different roles working together"

**Examples**: ChatDev (CEO, designer, tester roles), research teams.

**Our API mapping**:
```rust
InjectionType::WorkerResult {
    worker_id: "security-analyst".into(),
    result: worker_findings,
}
```

---

## Context Engineering Strategies

From [LangChain's Context Engineering](https://blog.langchain.com/context-engineering-for-agents/):

### The Four Strategies

```
┌─────────────────────────────────────────────────────────────────┐
│                    CONTEXT WINDOW                               │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  System Prompt + Tools + Messages + Working Memory       │  │
│  └──────────────────────────────────────────────────────────┘  │
│         ▲              ▲              ▲              ▲         │
│         │              │              │              │         │
│    ┌────┴────┐    ┌────┴────┐   ┌────┴────┐    ┌────┴────┐   │
│    │  WRITE  │    │ SELECT  │   │COMPRESS │    │ ISOLATE │   │
│    │ Context │    │ Context │   │ Context │    │ Context │   │
│    └─────────┘    └─────────┘   └─────────┘    └─────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

#### 1. Write Context

Persist information outside the context window for future use.

**Implementations**:
- Scratchpads (tool-based note-taking)
- Memory files (persistent storage)
- State objects (runtime persistence)

**Our API**:
```rust
// Inject saved context back when needed
InjectionType::SystemContext(saved_notes)
InjectionType::MemorySummary(compressed_history)
```

#### 2. Select Context

Retrieve only relevant information into context.

**Implementations**:
- RAG (semantic search)
- Tool description filtering
- Knowledge graph traversal

**Our API**: Not directly injection (retrieval happens before injection)

#### 3. Compress Context

Reduce tokens while retaining essential information.

**Implementations**:
- Recursive summarization
- Hierarchical compression
- Auto-compact (Claude Code: triggers at 95% capacity)

**Our API**:
```rust
InjectionType::MemorySummary(compressed_summary)
InjectionTrigger::OnTokenThreshold(threshold)
```

#### 4. Isolate Context

Distribute across separate systems to prevent overload.

**Implementations**:
- Sub-agent architectures
- Specialized tool agents
- Parallel processing with aggregation

**Our API**:
```rust
InjectionType::WorkerResult { worker_id, result }
```

---

## Memory Management Patterns

### Memory Types

| Type | Scope | Persistence | Our Implementation |
|------|-------|-------------|-------------------|
| **Working Memory** | Single task | Ephemeral | Conversation history |
| **Short-term Memory** | Session | Session-scoped | TODO list, scratchpad |
| **Long-term Memory** | Cross-session | Persistent | CLAUDE.md, memory files |

### Working Memory (Scratchpads)

From [Mem0](https://mem0.ai/blog/memory-in-agents-what-why-and-how):

> "Just as humans jot notes while tackling complex problems, AI agents use scratchpads to preserve information for future reference."

**Implementation approaches**:
1. Tool-based saving (agent calls save_note tool)
2. Runtime state fields (persist across execution steps)
3. File-based storage (write to scratch directory)

**Use case for injection**:
```rust
// Inject scratchpad contents as context
InjectionType::SystemContext(format!(
    "[Working Notes]\n{}",
    scratchpad_contents
))
```

### Short-term Memory (Session State)

**Patterns**:
- Rolling buffer of recent messages
- TODO list tracking (Claude Code pattern)
- Intermediate step annotations

**Use case for injection**:
```rust
// Inject TODO state as reminder
InjectionType::SystemContext(format!(
    "[Current Tasks]\n- In Progress: {}\n- Remaining: {}",
    current_task,
    remaining_count
))
```

### Long-term Memory (Cross-session)

From Anthropic's memory tool:

> "The agent can write down facts or interim results to a file, and later retrieve them in a future conversation."

**Categories** (CoALA paper):
- **Episodic**: Past action sequences (few-shot examples)
- **Procedural**: How the agent works (CLAUDE.md, rules files)
- **Semantic**: Facts and knowledge (memory graphs)

---

## Claude Code Design Patterns

From [Agent Design Lessons](https://jannesklaas.github.io/ai/2025/07/20/claude-code-agent-design.html):

### TODO List as Memory

Claude Code uses built-in TODO lists for planning and state tracking:

```
TODO States: pending → in_progress → completed
```

**Key insight**: Tool results include reminders to "keep using the TODO list to track work."

**Injection pattern**:
```rust
// After each response, remind about TODO state
InjectionType::SystemContext(format!(
    "[TODO Status] {} in progress, {} remaining",
    in_progress_count,
    pending_count
))
```

### System Reminders for Context Persistence

> "Rather than sophisticated memory systems, Claude Code uses periodic system reminders injected at critical junctures."

**Pattern**: Repeat important instructions multiple times throughout session to combat "instruction decay."

**Injection pattern**:
```rust
// Periodic reminder injection
InjectionType::SystemContext(
    "[Reminder] Focus on the current task. \
     Avoid creating unnecessary files. \
     Update TODO list after each step.".into()
)
```

### Sub-Agent Dispatching

**Purpose**: Manage context window limits + enable parallelism.

**Pattern**: Sub-agents are identical instances that can't spawn further sub-agents.

**Injection pattern**:
```rust
// Inject sub-agent results back to parent
InjectionType::WorkerResult {
    worker_id: sub_agent_id,
    result: condensed_result,
}
```

### ROADMAP.md Pattern

> "A ROADMAP.md file acts as a single entry point to planning out features and tasks."

**Usage**: Include in CLAUDE.md via import syntax for persistent high-level context.

---

## Use Case Mapping to Our API

### Currently Implemented Use Cases

| Use Case | Injection Type | Trigger | Status |
|----------|---------------|---------|--------|
| Auto-analysis after tool | `ToolFollowUp` | `AfterResponse` | Ready |
| Error recovery prompts | `ErrorRecovery` | `Immediate` | Ready |
| Context summarization | `MemorySummary` | `OnTokenThreshold` | Ready |
| Multi-agent coordination | `WorkerResult` | `Immediate` | Ready |
| Agent redirection | `Correction` | `Immediate` | Ready |
| System context injection | `SystemContext` | Various | Ready |

### Patterns Ready for Implementation

#### Pattern 1: Reflection Loop

```rust
// After each response, trigger self-review
agent.queue_injection(
    InjectionType::Correction(
        "Before finalizing, review your response:\n\
         1. Are the facts accurate?\n\
         2. Did you address all parts of the request?\n\
         3. Is the format appropriate?\n\
         If issues found, provide corrected version.".into()
    ),
    InjectionTrigger::AfterResponse,
);
```

#### Pattern 2: TODO State Reminder

```rust
// Inject TODO status periodically
let todo_summary = format_todo_summary(&agent.todo_list());
agent.queue_injection(
    InjectionType::SystemContext(format!(
        "[Task Progress]\n{}\n\nContinue with the next pending task.",
        todo_summary
    )),
    InjectionTrigger::AfterTurns(3), // Every 3 turns
);
```

#### Pattern 3: Context Compaction

```rust
// Auto-compact when approaching limit
agent.queue_injection(
    InjectionType::MemorySummary(
        summarize_conversation(&agent.messages())
    ),
    InjectionTrigger::OnTokenThreshold(90_000), // 90% of 100k
);
```

#### Pattern 4: Tool Result Analysis

```rust
// After JavaScript execution, prompt analysis
agent.queue_injection(
    InjectionType::ToolFollowUp {
        tool_name: "execute_javascript".into(),
        context: "Analyze the returned data:\n\
                  1. Summarize key findings\n\
                  2. Identify any anomalies\n\
                  3. Suggest next steps".into(),
    },
    InjectionTrigger::AfterToolComplete {
        tool_name: "execute_javascript".into(),
        on_success_only: true,
    },
);
```

#### Pattern 5: Error Recovery with Alternatives

```rust
// On tool failure, suggest alternatives
agent.queue_injection(
    InjectionType::ErrorRecovery {
        error: "API rate limit exceeded for us-east-1".into(),
        suggestion: Some(
            "Try these alternatives:\n\
             1. Query us-west-2 region instead\n\
             2. Reduce batch size\n\
             3. Add delay between requests".into()
        ),
    },
    InjectionTrigger::AfterToolComplete {
        tool_name: "execute_javascript".into(),
        on_success_only: false, // Trigger on failure
    },
);
```

#### Pattern 6: Multi-Agent Result Synthesis

```rust
// Collect results from multiple workers
for worker in completed_workers {
    parent_agent.queue_immediate_injection(
        InjectionType::WorkerResult {
            worker_id: worker.id().to_string(),
            result: worker.get_summary(),
        }
    );
}

// Then request synthesis
parent_agent.queue_injection(
    InjectionType::SystemContext(
        "All worker agents have reported. \
         Synthesize their findings into a unified response.".into()
    ),
    InjectionTrigger::Immediate,
);
```

---

## Future Implementation Ideas

### 1. Scratchpad Integration

Create a scratchpad tool that agents can write to, with automatic injection of scratchpad contents at key points.

```rust
// Proposed new injection type
InjectionType::ScratchpadContents {
    section: Option<String>,  // Optional section filter
    format: ScratchpadFormat, // Raw, Summarized, Structured
}
```

### 2. TODO-Aware Injections

Integrate with agent's TODO list to provide contextual reminders.

```rust
// Proposed trigger
InjectionTrigger::OnTodoStateChange {
    from: TodoState::InProgress,
    to: TodoState::Completed,
}
```

### 3. Periodic Reminder System

Implement instruction decay prevention through periodic re-injection.

```rust
// Proposed trigger
InjectionTrigger::EveryNTurns(5)
InjectionTrigger::EveryNMinutes(10)
```

### 4. Conversation Branching

Save conversation state and inject it later for backtracking.

```rust
// Proposed injection type
InjectionType::ConversationCheckpoint {
    checkpoint_id: String,
    context: String,
}
```

### 5. Memory Graph Integration

Connect to external memory systems for fact retrieval.

```rust
// Proposed injection type
InjectionType::RetrievedMemory {
    query: String,
    memories: Vec<MemoryEntry>,
}
```

---

## Summary

Our Message Injection Engine aligns well with industry patterns:

| Industry Pattern | Our Support | Notes |
|-----------------|-------------|-------|
| Prompt Chaining | `AfterResponse` | Fully supported |
| Orchestrator-Workers | `WorkerResult` | Fully supported |
| Context Compacting | `MemorySummary` + `OnTokenThreshold` | Fully supported |
| Reflection | `Correction` + `AfterResponse` | Fully supported |
| Tool Follow-up | `AfterToolComplete` | Fully supported |
| Error Recovery | `ErrorRecovery` | Fully supported |
| TODO Tracking | `SystemContext` | Manual integration |
| Scratchpads | `SystemContext` | Manual integration |
| Periodic Reminders | `AfterTurns` | Basic support |

**Key insight from research**: The most successful agent implementations use simple, composable patterns rather than complex frameworks. Our injection system follows this philosophy.

---

## References

1. Anthropic. "Building Effective Agents." December 2024.
2. Andrew Ng. "Agentic Reasoning." Sequoia AI Ascent 2024.
3. LangChain. "Context Engineering for Agents." 2025.
4. Mem0. "Memory in Agents: What, Why and How." 2024.
5. Jannes Klaas. "Agent Design Lessons from Claude Code." July 2025.
6. arXiv:2404.13501. "A Survey on the Memory Mechanism of Large Language Model based Agents." April 2024.
