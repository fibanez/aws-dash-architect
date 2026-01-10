//! Auto Analysis Layer
//!
//! Middleware layer that automatically triggers follow-up analysis
//! when responses contain raw data that could benefit from summarization.

#![warn(clippy::all, rust_2018_idioms)]

use crate::app::agent_framework::middleware::{
    ConversationLayer, LayerContext, LayerResult, PostResponseAction,
};

/// Configuration for auto-analysis behavior
#[derive(Debug, Clone)]
pub struct AutoAnalysisConfig {
    /// Patterns that indicate raw data needing analysis
    pub data_patterns: Vec<String>,
    /// Patterns that indicate analysis is already present
    pub analysis_patterns: Vec<String>,
    /// The follow-up prompt to inject
    pub analysis_prompt: String,
    /// Minimum response length to consider for analysis
    pub min_response_length: usize,
    /// Whether auto-analysis is enabled
    pub enabled: bool,
}

impl Default for AutoAnalysisConfig {
    fn default() -> Self {
        Self {
            data_patterns: vec![
                "resources found".to_string(),
                "results:".to_string(),
                "items returned".to_string(),
                "data retrieved".to_string(),
                "records:".to_string(),
            ],
            analysis_patterns: vec![
                "Summary:".to_string(),
                "Analysis:".to_string(),
                "In summary".to_string(),
                "Key findings:".to_string(),
                "Overview:".to_string(),
            ],
            analysis_prompt: "Please provide a brief summary and analysis of these results, highlighting key findings and any notable patterns.".to_string(),
            min_response_length: 500,
            enabled: true,
        }
    }
}

impl AutoAnalysisConfig {
    /// Add a data pattern to trigger analysis
    pub fn with_data_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.data_patterns.push(pattern.into());
        self
    }

    /// Add an analysis pattern that indicates analysis is already present
    pub fn with_analysis_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.analysis_patterns.push(pattern.into());
        self
    }

    /// Set the analysis prompt
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.analysis_prompt = prompt.into();
        self
    }

    /// Set minimum response length
    pub fn with_min_length(mut self, length: usize) -> Self {
        self.min_response_length = length;
        self
    }

    /// Enable or disable auto-analysis
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Auto-analysis middleware layer
///
/// Detects responses containing raw data and automatically
/// triggers a follow-up request for analysis/summary.
///
/// ## How it works
///
/// 1. Checks if response contains any "data patterns" (e.g., "resources found")
/// 2. Checks if response already contains "analysis patterns" (e.g., "Summary:")
/// 3. If data is present but analysis is not, injects a follow-up prompt
///
/// ## Example
///
/// ```ignore
/// let layer = AutoAnalysisLayer::new(AutoAnalysisConfig::default());
/// stack.add(layer);
/// ```
pub struct AutoAnalysisLayer {
    config: AutoAnalysisConfig,
}

impl AutoAnalysisLayer {
    /// Create a new auto-analysis layer
    pub fn new(config: AutoAnalysisConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(AutoAnalysisConfig::default())
    }

    /// Check if response contains raw data patterns
    fn contains_data(&self, response: &str) -> bool {
        let lower = response.to_lowercase();
        self.config
            .data_patterns
            .iter()
            .any(|p| lower.contains(&p.to_lowercase()))
    }

    /// Check if response already contains analysis
    fn contains_analysis(&self, response: &str) -> bool {
        let lower = response.to_lowercase();
        self.config
            .analysis_patterns
            .iter()
            .any(|p| lower.contains(&p.to_lowercase()))
    }

    /// Check if response is long enough to warrant analysis
    fn is_long_enough(&self, response: &str) -> bool {
        response.len() >= self.config.min_response_length
    }

    /// Determine if analysis should be triggered
    fn should_analyze(&self, response: &str) -> bool {
        if !self.config.enabled {
            return false;
        }

        // Must be long enough
        if !self.is_long_enough(response) {
            return false;
        }

        // Must contain data patterns
        if !self.contains_data(response) {
            return false;
        }

        // Must NOT already contain analysis
        if self.contains_analysis(response) {
            return false;
        }

        true
    }
}

impl ConversationLayer for AutoAnalysisLayer {
    fn name(&self) -> &str {
        "AutoAnalysis"
    }

    fn on_post_response(
        &self,
        response: &str,
        _ctx: &LayerContext,
    ) -> LayerResult<PostResponseAction> {
        if self.should_analyze(response) {
            log::debug!("AutoAnalysis: Detected raw data, injecting analysis prompt");
            return Ok(PostResponseAction::InjectFollowUp(
                self.config.analysis_prompt.clone(),
            ));
        }

        Ok(PostResponseAction::PassThrough)
    }

    fn on_tool_complete(&self, tool_name: &str, success: bool, _ctx: &LayerContext) {
        // Log tool completions that might produce data
        if success
            && (tool_name.contains("query")
                || tool_name.contains("list")
                || tool_name.contains("get"))
        {
            log::trace!(
                "AutoAnalysis: Data-producing tool '{}' completed, watching for data patterns",
                tool_name
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::agent_framework::AgentType;

    #[test]
    fn test_config_default() {
        let config = AutoAnalysisConfig::default();
        assert!(!config.data_patterns.is_empty());
        assert!(!config.analysis_patterns.is_empty());
        assert!(!config.analysis_prompt.is_empty());
        assert!(config.enabled);
    }

    #[test]
    fn test_config_builder() {
        let config = AutoAnalysisConfig::default()
            .with_data_pattern("custom pattern")
            .with_analysis_pattern("Custom Analysis:")
            .with_prompt("Analyze this data")
            .with_min_length(1000)
            .enabled(false);

        assert!(config.data_patterns.contains(&"custom pattern".to_string()));
        assert!(config
            .analysis_patterns
            .contains(&"Custom Analysis:".to_string()));
        assert_eq!(config.analysis_prompt, "Analyze this data");
        assert_eq!(config.min_response_length, 1000);
        assert!(!config.enabled);
    }

    #[test]
    fn test_layer_creation() {
        let layer = AutoAnalysisLayer::with_defaults();
        assert_eq!(layer.name(), "AutoAnalysis");
    }

    #[test]
    fn test_contains_data() {
        let layer = AutoAnalysisLayer::with_defaults();

        assert!(layer.contains_data("Found 50 resources found in the account"));
        assert!(layer.contains_data("Here are the results:"));
        assert!(layer.contains_data("10 items returned from query"));
        assert!(!layer.contains_data("Hello, how can I help you?"));
    }

    #[test]
    fn test_contains_analysis() {
        let layer = AutoAnalysisLayer::with_defaults();

        assert!(layer.contains_analysis("Summary: Here are the key findings"));
        assert!(layer.contains_analysis("Analysis: The data shows..."));
        assert!(layer.contains_analysis("In summary, we found that..."));
        assert!(!layer.contains_analysis("Raw data output"));
    }

    #[test]
    fn test_should_analyze_raw_data() {
        let layer = AutoAnalysisLayer::new(AutoAnalysisConfig::default().with_min_length(10));

        // Long response with data but no analysis
        let response = "Here are 50 resources found in the account:\n\
            - Resource 1: EC2 instance\n\
            - Resource 2: S3 bucket\n\
            - Resource 3: Lambda function\n\
            ... (more data)";

        assert!(layer.should_analyze(response));
    }

    #[test]
    fn test_should_not_analyze_with_existing_analysis() {
        let layer = AutoAnalysisLayer::new(AutoAnalysisConfig::default().with_min_length(10));

        // Response with data AND analysis
        let response = "Here are 50 resources found in the account:\n\
            - Resource 1: EC2 instance\n\
            Summary: The account has a mix of compute and storage resources.";

        assert!(!layer.should_analyze(response));
    }

    #[test]
    fn test_should_not_analyze_short_response() {
        let layer = AutoAnalysisLayer::with_defaults();

        // Too short
        let response = "5 resources found";
        assert!(!layer.should_analyze(response));
    }

    #[test]
    fn test_should_not_analyze_when_disabled() {
        let layer = AutoAnalysisLayer::new(AutoAnalysisConfig::default().enabled(false));

        let response = "Here are 50 resources found in the account with lots of data...";
        assert!(!layer.should_analyze(response));
    }

    #[test]
    fn test_post_response_triggers_injection() {
        let layer = AutoAnalysisLayer::new(AutoAnalysisConfig::default().with_min_length(10));
        let ctx = LayerContext::new("test", AgentType::TaskManager);

        let response = "Here are 50 resources found in the account:\n\
            Lots of raw data here without any summary...";

        let result = layer.on_post_response(response, &ctx).unwrap();

        match result {
            PostResponseAction::InjectFollowUp(prompt) => {
                assert!(prompt.contains("summary"));
            }
            _ => panic!("Expected InjectFollowUp action"),
        }
    }

    #[test]
    fn test_post_response_passthrough() {
        let layer = AutoAnalysisLayer::with_defaults();
        let ctx = LayerContext::new("test", AgentType::TaskManager);

        // Simple response without data
        let response = "Hello! I can help you with AWS resources.";

        let result = layer.on_post_response(response, &ctx).unwrap();
        assert!(matches!(result, PostResponseAction::PassThrough));
    }
}
