//! Performance Tracing Utilities for Agent Creation
//!
//! This module provides structured performance timing and bottleneck analysis
//! for agent creation operations to identify performance issues.

use std::time::{Duration, Instant};
use tracing::{info, warn};

/// Performance timer for measuring operation durations
pub struct PerformanceTimer {
    operation_name: String,
    start_time: Instant,
    phase_times: Vec<(String, Duration)>,
    current_phase: Option<(String, Instant)>,
}

impl PerformanceTimer {
    /// Create a new performance timer for an operation
    pub fn new(operation_name: &str) -> Self {
        info!("🚀 PERF: {} started", operation_name);
        Self {
            operation_name: operation_name.to_string(),
            start_time: Instant::now(),
            phase_times: Vec::new(),
            current_phase: None,
        }
    }

    /// Start timing a specific phase
    pub fn start_phase(&mut self, phase_name: &str) {
        // End current phase if one is active
        if let Some((current_name, current_start)) = self.current_phase.take() {
            let duration = current_start.elapsed();
            self.phase_times.push((current_name, duration));
        }

        // Start new phase
        self.current_phase = Some((phase_name.to_string(), Instant::now()));
    }

    /// End the current phase and log its duration
    pub fn end_phase(&mut self) {
        if let Some((phase_name, phase_start)) = self.current_phase.take() {
            let duration = phase_start.elapsed();

            // Log phase completion with duration
            if duration.as_millis() > 100 {
                warn!(
                    "🐌 PERF: {} - {}ms ⚠️ SLOW",
                    phase_name,
                    duration.as_millis()
                );
            } else {
                info!("⚡ PERF: {} - {}ms", phase_name, duration.as_millis());
            }

            self.phase_times.push((phase_name, duration));
        }
    }

    /// Complete the operation and log comprehensive timing analysis
    pub fn complete(mut self) {
        // End any remaining phase
        self.end_phase();

        let total_duration = self.start_time.elapsed();
        let total_ms = total_duration.as_millis();

        info!(
            "✅ PERF: {} completed - {}ms",
            self.operation_name, total_ms
        );

        // Performance analysis
        if total_ms > 500 {
            warn!(
                "🚨 PERF: {} took {}ms (expected: ~500ms) - PERFORMANCE ISSUE",
                self.operation_name, total_ms
            );
        }

        // Bottleneck analysis
        if !self.phase_times.is_empty() {
            info!("📊 PERF: {} phase breakdown:", self.operation_name);

            let mut sorted_phases = self.phase_times.clone();
            sorted_phases.sort_by(|a, b| b.1.cmp(&a.1)); // Sort by duration descending

            for (phase_name, duration) in &sorted_phases {
                let percentage = (duration.as_millis() as f64 / total_ms as f64) * 100.0;
                let icon = if percentage > 30.0 {
                    "🐌"
                } else if percentage > 10.0 {
                    "⏳"
                } else {
                    "⚡"
                };

                info!(
                    "📊 PERF:   {} {} - {}ms ({:.1}%)",
                    icon,
                    phase_name,
                    duration.as_millis(),
                    percentage
                );
            }

            // Identify primary bottleneck
            if let Some((bottleneck_phase, bottleneck_duration)) = sorted_phases.first() {
                let bottleneck_percentage =
                    (bottleneck_duration.as_millis() as f64 / total_ms as f64) * 100.0;
                if bottleneck_percentage > 30.0 {
                    warn!(
                        "🎯 PERF: Primary bottleneck: {} ({:.1}% of total time)",
                        bottleneck_phase, bottleneck_percentage
                    );
                }
            }
        }

        // Memory and resource usage hints
        if total_ms > 2000 {
            info!("💡 PERF: Consider optimizing: check network latency, model initialization, credential caching");
        }
    }
}

/// Convenience macro for timing a block of code
#[macro_export]
macro_rules! time_phase {
    ($timer:expr, $phase_name:expr, $code:block) => {{
        $timer.start_phase($phase_name);
        let result = $code;
        $timer.end_phase();
        result
    }};
}

/// Performance metrics for agent creation
#[derive(Debug, Clone)]
pub struct AgentCreationMetrics {
    pub agent_type: String,
    pub agent_id: String,
    pub total_duration: Duration,
    pub validation_duration: Duration,
    pub credential_duration: Duration,
    pub builder_setup_duration: Duration,
    pub agent_build_duration: Duration,
    pub execution_duration: Duration,
    pub success: bool,
}

impl AgentCreationMetrics {
    /// Log structured metrics to the log file
    pub fn log_structured(&self) {
        info!(
            "PERF_METRICS: agent_type={}, agent_id={}, total_ms={}, validation_ms={}, credential_ms={}, builder_ms={}, build_ms={}, execution_ms={}, success={}",
            self.agent_type,
            &self.agent_id[..8], // Only log first 8 characters of agent ID
            self.total_duration.as_millis(),
            self.validation_duration.as_millis(),
            self.credential_duration.as_millis(),
            self.builder_setup_duration.as_millis(),
            self.agent_build_duration.as_millis(),
            self.execution_duration.as_millis(),
            self.success
        );
    }

    /// Analyze and warn about performance issues
    pub fn analyze_performance(&self) {
        let total_ms = self.total_duration.as_millis();

        if total_ms > 5000 {
            warn!(
                "🚨 SEVERE: Agent creation took {}ms (>5s) - agent_type={}",
                total_ms, self.agent_type
            );
        } else if total_ms > 2000 {
            warn!(
                "⚠️ SLOW: Agent creation took {}ms (>2s) - agent_type={}",
                total_ms, self.agent_type
            );
        }

        // Specific phase warnings
        if self.agent_build_duration.as_millis() > 1000 {
            warn!(
                "🐌 Agent.build() is slow: {}ms - check model initialization",
                self.agent_build_duration.as_millis()
            );
        }

        if self.execution_duration.as_millis() > 3000 {
            warn!(
                "🐌 Agent execution is slow: {}ms - check LLM response time",
                self.execution_duration.as_millis()
            );
        }

        if self.credential_duration.as_millis() > 500 {
            warn!(
                "🐌 Credential retrieval is slow: {}ms - check AWS network latency",
                self.credential_duration.as_millis()
            );
        }
    }
}
