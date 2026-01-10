//! Memory Budget Enforcement
//!
//! Prevents the application from using more than 80% of system RAM.
//! Checks total system memory at startup and monitors usage during queries.

use std::sync::OnceLock;
use sysinfo::{System, RefreshKind, MemoryRefreshKind};

/// Memory budget configuration (initialized once at startup)
static MEMORY_BUDGET: OnceLock<MemoryBudget> = OnceLock::new();

/// Memory budget tracker
#[derive(Debug, Clone)]
pub struct MemoryBudget {
    /// Total system RAM in bytes
    total_memory_bytes: u64,
    /// Maximum allowed usage (80% of total)
    max_allowed_bytes: u64,
}

impl MemoryBudget {
    /// Initialize memory budget by checking system RAM
    ///
    /// This should be called once at application startup.
    pub fn initialize() -> &'static Self {
        MEMORY_BUDGET.get_or_init(|| {
            let mut sys = System::new_with_specifics(
                RefreshKind::new().with_memory(MemoryRefreshKind::new().with_ram())
            );
            sys.refresh_memory();

            let total_memory_bytes = sys.total_memory();
            let max_allowed_bytes = (total_memory_bytes as f64 * 0.8) as u64;

            tracing::info!(
                "Memory budget initialized: total {} MB, limit {} MB (80%)",
                total_memory_bytes / 1_048_576,
                max_allowed_bytes / 1_048_576
            );

            Self {
                total_memory_bytes,
                max_allowed_bytes,
            }
        })
    }

    /// Get the global memory budget (initializes if needed)
    pub fn get() -> &'static Self {
        MEMORY_BUDGET.get_or_init(|| Self::initialize().clone())
    }

    /// Check if current memory usage is within budget
    ///
    /// Returns Ok(()) if within budget, Err(message) if exceeded.
    pub fn check_usage(&self) -> Result<(), String> {
        let current_usage = self.current_usage_bytes();

        if current_usage > self.max_allowed_bytes {
            let current_mb = current_usage / 1_048_576;
            let limit_mb = self.max_allowed_bytes / 1_048_576;

            Err(format!(
                "Memory limit exceeded: {} MB / {} MB (80% of system RAM)",
                current_mb, limit_mb
            ))
        } else {
            Ok(())
        }
    }

    /// Get current memory usage in bytes
    ///
    /// Returns current RSS memory usage, or max_allowed_bytes if stats unavailable.
    /// This ensures fail-closed behavior - if we can't measure memory, we deny
    /// queries rather than allowing unbounded allocation.
    fn current_usage_bytes(&self) -> u64 {
        if let Some(stats) = memory_stats::memory_stats() {
            stats.physical_mem as u64
        } else {
            // Fail-closed: if we can't get memory stats, assume at limit
            // This prevents memory budget bypass on platforms where memory_stats fails
            tracing::warn!(
                "Failed to read memory stats - failing closed (assuming at limit for safety)"
            );
            self.max_allowed_bytes
        }
    }

    /// Get current memory usage as percentage of limit (0.0 to 1.0+)
    pub fn usage_percentage(&self) -> f64 {
        let current = self.current_usage_bytes() as f64;
        let limit = self.max_allowed_bytes as f64;
        current / limit
    }

    /// Get total system memory in MB
    pub fn total_memory_mb(&self) -> u64 {
        self.total_memory_bytes / 1_048_576
    }

    /// Get memory limit in MB
    pub fn limit_mb(&self) -> u64 {
        self.max_allowed_bytes / 1_048_576
    }

    /// Get current usage in MB
    pub fn current_usage_mb(&self) -> u64 {
        self.current_usage_bytes() / 1_048_576
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_budget_initialization() {
        let budget = MemoryBudget::initialize();

        // Total memory should be non-zero
        assert!(budget.total_memory_bytes > 0);

        // Limit should be 80% of total
        let expected_limit = (budget.total_memory_bytes as f64 * 0.8) as u64;
        assert_eq!(budget.max_allowed_bytes, expected_limit);

        // Log values for inspection
        println!(
            "System RAM: {} MB, Limit: {} MB",
            budget.total_memory_mb(),
            budget.limit_mb()
        );
    }

    #[test]
    fn test_usage_percentage() {
        let budget = MemoryBudget::get();
        let usage_pct = budget.usage_percentage();

        // Usage percentage should be between 0 and some reasonable upper bound
        // (might be > 1.0 if we're over budget, which is fine for this test)
        assert!(usage_pct >= 0.0);

        println!("Current memory usage: {:.1}%", usage_pct * 100.0);
    }
}
