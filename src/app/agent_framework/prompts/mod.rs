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
pub mod page_builder_prompt;
pub mod page_builder_worker;
pub mod page_builder_common;
pub mod page_builder_results;
pub mod page_builder_tool;

// Re-export prompts as constants
pub use task_manager::TASK_MANAGER_PROMPT;
pub use task_worker::TASK_WORKER_PROMPT;
pub use page_builder_prompt::PAGE_BUILDER_PROMPT;
pub use page_builder_worker::PAGE_BUILDER_WORKER_PROMPT;
pub use page_builder_common::PAGE_BUILDER_COMMON;
pub use page_builder_results::PAGE_BUILDER_RESULTS_PROMPT;
pub use page_builder_tool::PAGE_BUILDER_TOOL_PROMPT;

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
    fn test_page_builder_prompt_exists() {
        assert!(!PAGE_BUILDER_PROMPT.is_empty());
        assert!(PAGE_BUILDER_PROMPT.contains("Page Builder Agent"));
        assert!(PAGE_BUILDER_PROMPT.contains("dashApp API"));
    }

    #[test]
    fn test_page_builder_common_prompt_exists() {
        assert!(!PAGE_BUILDER_COMMON.is_empty());
        assert!(PAGE_BUILDER_COMMON.contains("VFS"));
        assert!(PAGE_BUILDER_COMMON.contains("dashApp"));
        assert!(PAGE_BUILDER_COMMON.contains("index.html"));
    }

    #[test]
    fn test_page_builder_results_prompt_exists() {
        assert!(!PAGE_BUILDER_RESULTS_PROMPT.is_empty());
        assert!(PAGE_BUILDER_RESULTS_PROMPT.contains("DISPLAY"));
        assert!(PAGE_BUILDER_RESULTS_PROMPT.contains("VFS"));
    }

    #[test]
    fn test_page_builder_tool_prompt_exists() {
        assert!(!PAGE_BUILDER_TOOL_PROMPT.is_empty());
        assert!(PAGE_BUILDER_TOOL_PROMPT.contains("REUSABLE TOOL"));
        assert!(PAGE_BUILDER_TOOL_PROMPT.contains("dashApp"));
    }

    #[test]
    fn test_prompts_are_different() {
        assert_ne!(TASK_MANAGER_PROMPT, TASK_WORKER_PROMPT);
        assert_ne!(TASK_MANAGER_PROMPT, PAGE_BUILDER_PROMPT);
        assert_ne!(TASK_WORKER_PROMPT, PAGE_BUILDER_PROMPT);
        assert_ne!(PAGE_BUILDER_RESULTS_PROMPT, PAGE_BUILDER_TOOL_PROMPT);
    }
}
