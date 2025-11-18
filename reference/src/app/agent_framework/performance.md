# Performance - Agent Creation Timing and Metrics

## Component Overview

Performance tracing utilities for measuring and analyzing agent creation
bottlenecks. Provides phase-by-phase timing, bottleneck identification, and
structured metrics logging for optimization.

**Pattern**: Performance timer with phase tracking
**Algorithm**: Instant::now() + Duration calculations
**External**: std::time::{Instant, Duration}, tracing

---

## Major Types

- `PerformanceTimer` - Phase-based timer with bottleneck analysis
- `AgentCreationMetrics` - Structured metrics for agent creation phases

---

## Major Methods

### PerformanceTimer
- `new()` - Start timer for operation
- `start_phase()` - Begin timing phase, end previous phase
- `end_phase()` - Complete current phase, log duration
- `complete()` - Finish operation, log analysis and bottlenecks

### AgentCreationMetrics
- `log_structured()` - Log metrics in structured format for parsing
- `analyze_performance()` - Identify slow phases, warn on issues

---

## Implementation Patterns

### Pattern: Phase-Based Timing

**Algorithm**: Stack-based phase tracking with automatic transitions
**External**: Instant::elapsed() for microsecond precision

Pseudocode:
  1. Create PerformanceTimer with operation name
  2. start_phase("validation"):
     - End previous phase if active (store duration)
     - Start new phase with Instant::now()
  3. end_phase():
     - Calculate phase duration
     - Log with emoji: ⚡ fast (<100ms) or 🐌 slow (>100ms)
     - Store in phase_times Vec
  4. Repeat for each phase
  5. complete():
     - End remaining phase
     - Calculate total duration
     - Sort phases by duration (descending)
     - Log breakdown with percentages
     - Identify bottleneck (>30% of total time)

### Pattern: Bottleneck Analysis

**Algorithm**: Sort phases by duration, calculate percentages
**External**: Vec::sort_by with Duration comparisons

Pseudocode:
  1. Collect all phase durations in Vec
  2. Sort descending by duration
  3. For each phase:
     percentage = (phase_ms / total_ms) * 100.0
     icon = if >30%: 🐌 else if >10%: ⏳ else: ⚡
     log "icon phase_name - duration (percentage%)"
  4. Primary bottleneck: first in sorted list if >30%
  5. Suggests optimization targets

### Pattern: Structured Metrics Logging

**Algorithm**: Key=value logging for automated parsing
**External**: tracing::info! macro

Pseudocode:
  1. AgentCreationMetrics captures:
     - agent_type, agent_id (first 8 chars)
     - total, validation, credential, builder, build, execution durations
     - success boolean
  2. log_structured() outputs:
     "PERF_METRICS: agent_type=X, agent_id=Y, total_ms=Z..."
  3. Parseable by log analysis tools
  4. Enables performance monitoring dashboards

---

## External Dependencies

- **std::time**: Instant, Duration for high-precision timing
- **tracing**: info!/warn! macros for logging
- **time_phase! macro**: Convenience macro for timing code blocks

---

## Key Algorithms

### Performance Thresholds
- Fast: <500ms total agent creation
- Slow: >500ms (warns)
- Severe: >2000ms (critical warning)
- Phase slow: >100ms individual phase

### Optimization Suggestions
Based on bottleneck analysis:
- agent_build_duration >1000ms → check model initialization
- execution_duration >3000ms → check LLM response time
- credential_duration >500ms → check AWS network latency
- total >2000ms → check network, model init, credential caching

---

**Last Updated**: 2025-01-28
**Status**: Accurately reflects performance.rs implementation
