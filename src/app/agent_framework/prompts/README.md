# Agent System Prompts

This directory contains system prompts for different agent types.

## Structure

- `task_manager.rs` - System prompt for TaskManager agents (orchestrators)
- `task_worker.rs` - System prompt for TaskWorker agents (executors)
- `mod.rs` - Module root with re-exports

## Prompt Engineering Principles

Based on Anthropic's research and best practices:

1. **Autonomous Operation** - Agents operate in self-directed loops without human intervention
2. **Self-Talk** - Agents reason to themselves, not to humans ("I need to" not "I will for you")
3. **XML Structure** - Use tags like `<thinking>`, `<summary>`, `<result>` for parseable outputs
4. **Role Clarity** - Managers coordinate, workers execute
5. **Complete Data** - Workers return full data in `<result>`, not summaries
6. **Maximize JavaScript Power** - Combine operations (filter, sort, aggregate) in ONE task when possible

## Tool Descriptions

Tool descriptions are co-located with their implementations:

- `../tools/execute_javascript.rs` - JavaScript execution tool (133 lines of API docs)
- `../tools/start_task.rs` - Worker spawning tool
- `../tools/think.rs` - Reasoning tool (no-op)

## References

- [Anthropic: Building Effective AI Agents](https://www.anthropic.com/research/building-effective-agents)
- [Anthropic: Multi-Agent Research System](https://www.anthropic.com/engineering/multi-agent-research-system)
- [Anthropic: XML Tags for Prompts](https://docs.claude.com/en/docs/build-with-claude/prompt-engineering/use-xml-tags)
