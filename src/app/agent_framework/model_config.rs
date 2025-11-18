//! Model Configuration for AI Agent Selection
//!
//! Provides model definitions and configuration for the Agent Framework system.
//! Supports various AI models including Claude 3.5 Sonnet, Haiku, and Amazon Nova series.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Macro to create agent with the correct model based on model ID
#[macro_export]
macro_rules! create_agent_with_model {
    ($agent_builder:expr, $model_id:expr) => {
        {
            use stood::llm::Bedrock::*;
            match $model_id.as_str() {
                "anthropic.claude-3-5-sonnet-20241022-v2:0" => $agent_builder.model(Claude35Sonnet),
                "anthropic.claude-3-5-haiku-20241022-v1:0" => $agent_builder.model(ClaudeHaiku3),
                "amazon.nova-micro-v1:0" => $agent_builder.model(NovaMicro),
                "amazon.nova-lite-v1:0" => $agent_builder.model(NovaLite),
                "amazon.nova-pro-v1:0" => $agent_builder.model(NovaPro),
                _ => $agent_builder.model(Claude35Sonnet), // Default to Claude 3.5 Sonnet
            }
        }
    };
}

/// Configuration for an AI model available in the Agent Framework system
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelConfig {
    /// Bedrock model ID (e.g., "anthropic.claude-3-5-sonnet-20241022-v2:0")
    pub model_id: String,
    /// Human-readable display name (e.g., "Claude 3.5 Sonnet")
    pub display_name: String,
    /// Model provider (e.g., "Anthropic", "Amazon")
    pub provider: String,
    /// Model description for tooltips
    pub description: String,
    /// Whether this model is currently available
    pub available: bool,
}

impl ModelConfig {
    /// Get the default model configurations supported by the Agent Framework system
    pub fn default_models() -> Vec<ModelConfig> {
        vec![
            ModelConfig {
                model_id: "anthropic.claude-3-5-sonnet-20241022-v2:0".to_string(),
                display_name: "Claude 3.5 Sonnet".to_string(),
                provider: "Anthropic".to_string(),
                description: "Most capable model for complex reasoning and analysis".to_string(),
                available: true,
            },
            ModelConfig {
                model_id: "anthropic.claude-3-5-haiku-20241022-v1:0".to_string(),
                display_name: "Claude 3.5 Haiku".to_string(),
                provider: "Anthropic".to_string(),
                description: "Fast and efficient model for quick tasks".to_string(),
                available: true,
            },
            ModelConfig {
                model_id: "amazon.nova-micro-v1:0".to_string(),
                display_name: "Amazon Nova Micro".to_string(),
                provider: "Amazon".to_string(),
                description: "Lightweight model optimized for speed and cost".to_string(),
                available: true,
            },
            ModelConfig {
                model_id: "amazon.nova-lite-v1:0".to_string(),
                display_name: "Amazon Nova Lite".to_string(),
                provider: "Amazon".to_string(),
                description: "Balanced model for general-purpose tasks".to_string(),
                available: true,
            },
            ModelConfig {
                model_id: "amazon.nova-pro-v1:0".to_string(),
                display_name: "Amazon Nova Pro".to_string(),
                provider: "Amazon".to_string(),
                description: "High-performance model for complex tasks".to_string(),
                available: true,
            },
        ]
    }

    /// Get the default model (Claude 3.5 Sonnet)
    pub fn default_model_id() -> String {
        "anthropic.claude-3-5-sonnet-20241022-v2:0".to_string()
    }

    /// Find a model configuration by model ID
    pub fn find_by_id<'a>(models: &'a [ModelConfig], model_id: &str) -> Option<&'a ModelConfig> {
        models.iter().find(|m| m.model_id == model_id)
    }

    /// Get display name for a model ID, with fallback
    pub fn get_display_name(models: &[ModelConfig], model_id: &str) -> String {
        Self::find_by_id(models, model_id)
            .map(|m| m.display_name.clone())
            .unwrap_or_else(|| format!("Unknown Model ({})", model_id))
    }

    /// Group models by provider for organized display
    pub fn group_by_provider(models: &[ModelConfig]) -> HashMap<String, Vec<&ModelConfig>> {
        let mut grouped = HashMap::new();
        for model in models {
            grouped
                .entry(model.provider.clone())
                .or_insert_with(Vec::new)
                .push(model);
        }
        grouped
    }

}

/// Settings for model preferences and configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSettings {
    /// Currently selected model ID
    pub selected_model: String,
    /// User preferences per model (e.g., temperature, max_tokens)
    pub model_preferences: HashMap<String, ModelPreference>,
    /// Whether to automatically switch to available models if selected is unavailable
    pub auto_fallback: bool,
}

/// Per-model preference settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPreference {
    /// Model temperature (0.0 - 1.0)
    pub temperature: Option<f32>,
    /// Maximum tokens for response
    pub max_tokens: Option<u32>,
    /// Whether this model is marked as favorite
    pub is_favorite: bool,
}

impl Default for ModelSettings {
    fn default() -> Self {
        Self {
            selected_model: ModelConfig::default_model_id(),
            model_preferences: HashMap::new(),
            auto_fallback: true,
        }
    }
}

impl ModelSettings {
    /// Get the currently selected model configuration
    pub fn get_selected_model<'a>(
        &self,
        available_models: &'a [ModelConfig],
    ) -> Option<&'a ModelConfig> {
        ModelConfig::find_by_id(available_models, &self.selected_model)
    }

    /// Set the selected model with validation
    pub fn set_selected_model(
        &mut self,
        model_id: String,
        available_models: &[ModelConfig],
    ) -> bool {
        if ModelConfig::find_by_id(available_models, &model_id).is_some() {
            self.selected_model = model_id;
            true
        } else {
            false
        }
    }

    /// Get fallback model if selected is unavailable
    pub fn get_fallback_model<'a>(
        &self,
        available_models: &'a [ModelConfig],
    ) -> Option<&'a ModelConfig> {
        if !self.auto_fallback {
            return None;
        }

        // Try to find first available model in preference order (Sonnet first as default)
        let preference_order = [
            "anthropic.claude-3-5-sonnet-20241022-v2:0",
            "anthropic.claude-3-5-haiku-20241022-v1:0",
            "amazon.nova-pro-v1:0",
            "amazon.nova-lite-v1:0",
            "amazon.nova-micro-v1:0",
        ];

        for model_id in &preference_order {
            if let Some(model) = ModelConfig::find_by_id(available_models, model_id) {
                if model.available {
                    return Some(model);
                }
            }
        }

        // If no preferred models available, return first available
        available_models.iter().find(|m| m.available)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_models() {
        let models = ModelConfig::default_models();
        assert!(!models.is_empty());

        // Check that we have the expected models
        let model_ids: Vec<String> = models.iter().map(|m| m.model_id.clone()).collect();
        assert!(model_ids.contains(&"anthropic.claude-3-5-sonnet-20241022-v2:0".to_string()));
        assert!(model_ids.contains(&"amazon.nova-pro-v1:0".to_string()));
    }

    #[test]
    fn test_find_by_id() {
        let models = ModelConfig::default_models();
        let model = ModelConfig::find_by_id(&models, "anthropic.claude-3-5-sonnet-20241022-v2:0");
        assert!(model.is_some());
        assert_eq!(model.unwrap().display_name, "Claude 3.5 Sonnet");
    }

    #[test]
    fn test_group_by_provider() {
        let models = ModelConfig::default_models();
        let grouped = ModelConfig::group_by_provider(&models);

        assert!(grouped.contains_key("Anthropic"));
        assert!(grouped.contains_key("Amazon"));
        assert!(grouped["Anthropic"].len() >= 2); // Sonnet and Haiku
        assert!(grouped["Amazon"].len() >= 3); // Nova variants
    }

    #[test]
    fn test_model_settings() {
        let models = ModelConfig::default_models();
        let mut settings = ModelSettings::default();

        // Test default selection
        assert!(settings.get_selected_model(&models).is_some());

        // Test changing selection
        assert!(settings.set_selected_model("amazon.nova-pro-v1:0".to_string(), &models));
        assert_eq!(settings.selected_model, "amazon.nova-pro-v1:0");

        // Test invalid selection
        assert!(!settings.set_selected_model("invalid-model".to_string(), &models));
    }

    #[test]
    fn test_fallback_model() {
        let models = ModelConfig::default_models();
        let settings = ModelSettings::default();

        let fallback = settings.get_fallback_model(&models);
        assert!(fallback.is_some());
    }
}
