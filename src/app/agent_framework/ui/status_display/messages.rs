//! Status Message Generator
//!
//! Generates whimsical, informative status messages for agent processing phases.
//!
//! ## Design
//!
//! - Messages are rotated to avoid monotony
//! - Each phase has a themed set of messages
//! - Optional details can be appended (e.g., token counts, tool names)

#![warn(clippy::all, rust_2018_idioms)]

use std::time::{Duration, Instant};

/// Processing phase of the agent
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessingPhase {
    /// Agent is thinking/reasoning (model call in progress)
    Thinking,

    /// Agent is executing a specific tool
    ExecutingTool(String),

    /// Agent is analyzing results from tool execution
    AnalyzingResults,

    /// Agent is idle (not processing)
    Idle,
}

impl Default for ProcessingPhase {
    fn default() -> Self {
        Self::Idle
    }
}

impl ProcessingPhase {
    /// Get a short label for the phase (for UI display)
    pub fn label(&self) -> &str {
        match self {
            ProcessingPhase::Thinking => "Thinking",
            ProcessingPhase::ExecutingTool(_) => "Executing",
            ProcessingPhase::AnalyzingResults => "Analyzing",
            ProcessingPhase::Idle => "Ready",
        }
    }

    /// Check if this is an active processing phase
    pub fn is_active(&self) -> bool {
        !matches!(self, ProcessingPhase::Idle)
    }
}

/// Generates whimsical status messages for processing phases
pub struct StatusMessageGenerator {
    /// Current message index for rotation
    current_index: usize,
    /// Last time message was changed
    last_change: Instant,
    /// Duration between message changes
    rotation_interval: Duration,
}

impl Default for StatusMessageGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusMessageGenerator {
    /// Message rotation interval (4 seconds feels natural)
    const DEFAULT_ROTATION_INTERVAL: Duration = Duration::from_secs(4);

    /// Create a new message generator
    pub fn new() -> Self {
        Self {
            current_index: 0,
            last_change: Instant::now(),
            rotation_interval: Self::DEFAULT_ROTATION_INTERVAL,
        }
    }

    /// Create with custom rotation interval
    pub fn with_rotation_interval(mut self, interval: Duration) -> Self {
        self.rotation_interval = interval;
        self
    }

    /// Get messages for the Thinking phase
    /// Inspired by Claude Code's whimsical status messages
    fn thinking_messages() -> &'static [&'static str] {
        &[
            // Classic contemplation
            "Pondering possibilities",
            "Cogitating carefully",
            "Musing methodically",
            "Contemplating options",
            "Ruminating thoughtfully",
            "Deliberating deeply",
            "Considering alternatives",
            "Weighing approaches",
            // Claude Code inspired
            "Accomplishing",
            "Actioning",
            "Actualizing",
            "Baking thoughts",
            "Brewing ideas",
            "Calculating",
            "Cerebrating",
            "Churning neurons",
            "Coalescing concepts",
            "Computing possibilities",
            "Conjuring solutions",
            "Constructing",
            "Crafting",
            "Crystallizing",
            "Deciphering",
            "Devising",
            "Digesting",
            "Elaborating",
            "Engaging circuits",
            "Engineering",
            "Envisioning",
            "Evaluating",
            "Examining",
            "Expediting",
            "Exploring",
            "Fabricating",
            "Formulating",
            "Generating",
            "Germinating ideas",
            "Hatching plans",
            "Ideating",
            "Imagining",
            "Incubating",
            "Iterating",
            "Manifesting",
            "Marinating thoughts",
            "Meditating",
            "Noodling",
            "Orchestrating",
            "Percolating",
            "Processing",
            "Reasoning",
            "Reticulating splines",
            "Scheming",
            "Simulating",
            "Speculating",
            "Strategizing",
            "Synthesizing",
            "Theorizing",
            "Transmuting",
            "Unraveling",
            "Visualizing",
            "Working magic",
        ]
    }

    /// Get messages for the ExecutingTool phase
    /// AWS-themed and action-oriented
    fn tool_messages() -> &'static [&'static str] {
        &[
            // Classic AWS themed
            "Consulting the oracle",
            "Summoning data",
            "Channeling the cloud",
            "Invoking AWS powers",
            "Querying the ether",
            "Fetching treasures",
            "Gathering intelligence",
            "Mining for insights",
            // Action words
            "Accomplishing tasks",
            "Activating",
            "Actualizing requests",
            "Assembling results",
            "Bridging systems",
            "Commanding resources",
            "Communicating",
            "Connecting",
            "Coordinating",
            "Dispatching",
            "Engaging APIs",
            "Executing",
            "Expediting",
            "Extracting",
            "Facilitating",
            "Fetching",
            "Firing up engines",
            "Handshaking",
            "Harvesting data",
            "Implementing",
            "Initiating",
            "Interfacing",
            "Interrogating APIs",
            "Invoking",
            "Launching",
            "Loading",
            "Mobilizing",
            "Negotiating",
            "Operating",
            "Orchestrating calls",
            "Performing",
            "Pinging servers",
            "Proceeding",
            "Processing requests",
            "Pulling levers",
            "Pushing buttons",
            "Querying",
            "Reaching out",
            "Requesting",
            "Retrieving",
            "Running operations",
            "Seeking answers",
            "Sending signals",
            "Spinning up",
            "Streaming",
            "Tapping resources",
            "Transmitting",
            "Traversing clouds",
            "Unleashing",
            "Wrangling data",
        ]
    }

    /// Get messages for the AnalyzingResults phase
    /// Contemplative and synthesis-focused
    fn analysis_messages() -> &'static [&'static str] {
        &[
            // Classic synthesis
            "Distilling wisdom",
            "Synthesizing insights",
            "Crystallizing findings",
            "Weaving conclusions",
            "Assembling the puzzle",
            "Connecting the dots",
            "Interpreting signals",
            "Refining understanding",
            // Analysis words
            "Absorbing results",
            "Aggregating",
            "Aligning findings",
            "Assimilating",
            "Cataloging",
            "Collating",
            "Compiling",
            "Composing",
            "Comprehending",
            "Condensing",
            "Consolidating",
            "Contemplating results",
            "Correlating",
            "Curating",
            "Decoding",
            "Deducing",
            "Deriving meaning",
            "Digesting data",
            "Discerning patterns",
            "Evaluating",
            "Extracting essence",
            "Filtering",
            "Focusing",
            "Forming conclusions",
            "Fusing insights",
            "Harmonizing",
            "Illuminating",
            "Integrating",
            "Making sense",
            "Mapping patterns",
            "Merging perspectives",
            "Organizing thoughts",
            "Parsing",
            "Piecing together",
            "Processing findings",
            "Rationalizing",
            "Reconciling",
            "Reducing complexity",
            "Resolving",
            "Sifting through",
            "Sorting out",
            "Structuring",
            "Summarizing",
            "Translating results",
            "Understanding",
            "Unifying",
            "Unpacking",
        ]
    }

    /// Update the message index if rotation interval has passed
    fn maybe_rotate(&mut self, message_count: usize) {
        if self.last_change.elapsed() >= self.rotation_interval {
            self.current_index = (self.current_index + 1) % message_count;
            self.last_change = Instant::now();
        }
    }

    /// Reset rotation state (call when phase changes)
    pub fn reset(&mut self) {
        self.current_index = 0;
        self.last_change = Instant::now();
    }

    /// Generate a status message for the given phase
    ///
    /// # Arguments
    /// * `phase` - Current processing phase
    /// * `detail` - Optional detail to append (e.g., "2,500 tokens")
    ///
    /// # Returns
    /// A whimsical message like "Pondering possibilities... (2,500 tokens)"
    pub fn generate(&mut self, phase: &ProcessingPhase, detail: Option<&str>) -> String {
        let base_message = match phase {
            ProcessingPhase::Thinking => {
                let messages = Self::thinking_messages();
                self.maybe_rotate(messages.len());
                messages[self.current_index % messages.len()]
            }
            ProcessingPhase::ExecutingTool(tool_name) => {
                let messages = Self::tool_messages();
                self.maybe_rotate(messages.len());
                let base = messages[self.current_index % messages.len()];
                // Return early with tool name in parentheses
                return match detail {
                    Some(d) => format!("{}... ({}) [{}]", base, tool_name, d),
                    None => format!("{}... ({})", base, tool_name),
                };
            }
            ProcessingPhase::AnalyzingResults => {
                let messages = Self::analysis_messages();
                self.maybe_rotate(messages.len());
                messages[self.current_index % messages.len()]
            }
            ProcessingPhase::Idle => return "Ready".to_string(),
        };

        match detail {
            Some(d) => format!("{}... ({})", base_message, d),
            None => format!("{}...", base_message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_labels() {
        assert_eq!(ProcessingPhase::Thinking.label(), "Thinking");
        assert_eq!(
            ProcessingPhase::ExecutingTool("test".into()).label(),
            "Executing"
        );
        assert_eq!(ProcessingPhase::AnalyzingResults.label(), "Analyzing");
        assert_eq!(ProcessingPhase::Idle.label(), "Ready");
    }

    #[test]
    fn test_phase_is_active() {
        assert!(ProcessingPhase::Thinking.is_active());
        assert!(ProcessingPhase::ExecutingTool("test".into()).is_active());
        assert!(ProcessingPhase::AnalyzingResults.is_active());
        assert!(!ProcessingPhase::Idle.is_active());
    }

    #[test]
    fn test_message_generation() {
        let mut gen = StatusMessageGenerator::new();

        // Test thinking message
        let msg = gen.generate(&ProcessingPhase::Thinking, None);
        assert!(msg.ends_with("..."));
        assert!(msg.contains("Pondering")); // First message in rotation

        // Test with detail
        let msg = gen.generate(&ProcessingPhase::Thinking, Some("2,500 tokens"));
        assert!(msg.contains("2,500 tokens"));
    }

    #[test]
    fn test_tool_message_format() {
        let mut gen = StatusMessageGenerator::new();
        let msg = gen.generate(
            &ProcessingPhase::ExecutingTool("execute_javascript".into()),
            None,
        );
        assert!(msg.contains("execute_javascript"));
        assert!(msg.contains("...")); // Has ellipsis
    }

    #[test]
    fn test_tool_message_with_detail() {
        let mut gen = StatusMessageGenerator::new();
        let msg = gen.generate(
            &ProcessingPhase::ExecutingTool("describe_ec2".into()),
            Some("15 resources"),
        );
        assert!(msg.contains("describe_ec2"));
        assert!(msg.contains("15 resources"));
    }

    #[test]
    fn test_idle_phase() {
        let mut gen = StatusMessageGenerator::new();
        let msg = gen.generate(&ProcessingPhase::Idle, None);
        assert_eq!(msg, "Ready");
    }

    #[test]
    fn test_reset() {
        let mut gen = StatusMessageGenerator::new();
        gen.current_index = 5;
        gen.reset();
        assert_eq!(gen.current_index, 0);
    }

    #[test]
    fn test_message_variety() {
        // Verify we have many messages per phase (50+ each, inspired by Claude Code)
        assert!(StatusMessageGenerator::thinking_messages().len() >= 50);
        assert!(StatusMessageGenerator::tool_messages().len() >= 50);
        assert!(StatusMessageGenerator::analysis_messages().len() >= 50);
    }

    #[test]
    fn test_message_count() {
        // Document exact counts for reference
        let thinking = StatusMessageGenerator::thinking_messages().len();
        let tool = StatusMessageGenerator::tool_messages().len();
        let analysis = StatusMessageGenerator::analysis_messages().len();
        let total = thinking + tool + analysis;

        // Should have 150+ total messages
        assert!(total >= 150, "Expected 150+ total messages, got {}", total);
        println!(
            "Message counts: Thinking={}, Tool={}, Analysis={}, Total={}",
            thinking, tool, analysis, total
        );
    }
}
