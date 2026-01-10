//! Model Selection for Agent Framework
//!
//! Provides a selection of Claude and Nova models that support tools and agents.
//! The actual model configuration is handled by stood - we just tell it which model to use.

#![warn(clippy::all, rust_2018_idioms)]

/// Supported models for agent creation
///
/// These are the Claude and Nova models available through Bedrock
/// that support tool use and agent capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgentModel {
    /// Claude Sonnet 4.5 - Balanced performance and cost
    #[default]
    ClaudeSonnet45,
    /// Claude Haiku 4.5 - Fast and cost-effective
    ClaudeHaiku45,
    /// Claude Opus 4.5 - Most capable
    ClaudeOpus45,
    /// Amazon Nova 2 Pro - intelligent reasoning model with 1M context
    Nova2Pro,
    /// Amazon Nova 2 Lite - fast reasoning model with 1M context
    Nova2Lite,
}

impl AgentModel {
    /// Get the display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            AgentModel::ClaudeSonnet45 => "Claude Sonnet 4.5",
            AgentModel::ClaudeHaiku45 => "Claude Haiku 4.5",
            AgentModel::ClaudeOpus45 => "Claude Opus 4.5",
            AgentModel::Nova2Pro => "Amazon Nova 2 Pro",
            AgentModel::Nova2Lite => "Amazon Nova 2 Lite",
        }
    }

    /// Get all available models for dropdown
    pub fn all_models() -> &'static [AgentModel] {
        &[
            AgentModel::ClaudeSonnet45,
            AgentModel::ClaudeHaiku45,
            AgentModel::ClaudeOpus45,
            AgentModel::Nova2Pro,
            AgentModel::Nova2Lite,
        ]
    }
}

impl std::fmt::Display for AgentModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_model() {
        let model = AgentModel::default();
        assert_eq!(model, AgentModel::ClaudeSonnet45);
    }

    #[test]
    fn test_display_names() {
        assert_eq!(
            AgentModel::ClaudeSonnet45.display_name(),
            "Claude Sonnet 4.5"
        );
        assert_eq!(AgentModel::ClaudeHaiku45.display_name(), "Claude Haiku 4.5");
        assert_eq!(AgentModel::ClaudeOpus45.display_name(), "Claude Opus 4.5");
        assert_eq!(AgentModel::Nova2Pro.display_name(), "Amazon Nova 2 Pro");
        assert_eq!(AgentModel::Nova2Lite.display_name(), "Amazon Nova 2 Lite");
    }

    #[test]
    fn test_all_models() {
        let models = AgentModel::all_models();
        assert_eq!(models.len(), 5);
    }

    #[test]
    fn test_display_trait() {
        assert_eq!(
            format!("{}", AgentModel::ClaudeSonnet45),
            "Claude Sonnet 4.5"
        );
    }
}
