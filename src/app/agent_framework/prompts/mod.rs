//! Agent System Prompts
//!
//! This module contains system prompts for different agent types.
//! Each agent type has a specialized prompt that defines its capabilities and behavior.
//!
//! Prompts follow Anthropic's best practices:
//! - Autonomous operation with self-talk
//! - XML tags for structured outputs (<summary>, <result>, <error>)
//! - Clear role definitions (coordinator vs. executor)
//! - Complete data in worker results
//! - Maximize JavaScript power (combine operations in single task)
//!
//! ## Research References
//!
//! - Building Effective AI Agents: https://www.anthropic.com/research/building-effective-agents
//! - Multi-Agent Orchestration: https://www.anthropic.com/engineering/multi-agent-research-system
//! - XML Tags for Prompts: https://docs.claude.com/en/docs/build-with-claude/prompt-engineering/use-xml-tags

pub mod task_manager;
pub mod task_worker;

// Re-export prompts as constants
pub use task_manager::TASK_MANAGER_PROMPT;
pub use task_worker::TASK_WORKER_PROMPT;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_manager_prompt_exists() {
        assert!(!TASK_MANAGER_PROMPT.is_empty());
        assert!(TASK_MANAGER_PROMPT.contains("autonomous"));
    }

    #[test]
    fn test_task_worker_prompt_exists() {
        assert!(!TASK_WORKER_PROMPT.is_empty());
        assert!(TASK_WORKER_PROMPT.contains("execute_javascript"));
    }

    #[test]
    fn test_prompts_are_different() {
        assert_ne!(TASK_MANAGER_PROMPT, TASK_WORKER_PROMPT);
    }
}
